use crate::{core::dataset::FaceInfo, result::PreprocessResult};
use gdal::{
    Dataset, DatasetOptions, GdalOpenFlags,
    raster::{Buffer, GdalType},
};
use itertools::izip;
use ndarray::{Axis, Zip};
use num::NumCast;
use std::collections::HashMap;
use terrain::math::{FaceRotation, NEIGHBOURING_FACES};

/// The four cardinal edges of a cube face, in the same order as `NEIGHBOUR_OFFSETS`'s first four
/// entries (up, right, down, left) and `NEIGHBOURING_FACES`'s index 1..4.
const EDGE_COUNT: usize = 4;

fn open_face(face_info: &FaceInfo) -> PreprocessResult<Dataset> {
    Ok(Dataset::open_ex(
        &face_info.path,
        DatasetOptions {
            open_flags: GdalOpenFlags::GDAL_OF_UPDATE,
            ..Default::default()
        },
    )?)
}

/// The offset/size of the single outermost row/column of *real* pixels for a given cardinal edge
/// of a `width`x`height` face raster - not a border/padding region, the actual rendered data.
fn edge_window(edge_index: usize, width: usize, height: usize) -> ((isize, isize), (usize, usize)) {
    match edge_index {
        0 => ((0, 0), (width, 1)),                     // up: row 0
        1 => ((width as isize - 1, 0), (1, height)),    // right: last column
        2 => ((0, height as isize - 1), (width, 1)),    // down: last row
        3 => ((0, 0), (1, height)),                     // left: column 0
        _ => unreachable!(),
    }
}

/// Rotates/transposes a buffer read from a neighbouring face so it lines up, pixel for pixel,
/// with this face's own edge orientation - the same transform `stitch.rs::neighbour_data` applies
/// when copying a neighbour's data into a tile's border, just reused here for whole-face edges.
fn align_to<T: Copy + GdalType>(buffer: Buffer<T>, rotation: &FaceRotation) -> PreprocessResult<Buffer<T>> {
    Ok(match rotation {
        FaceRotation::Identical | FaceRotation::ShiftU | FaceRotation::ShiftV => buffer,
        FaceRotation::RotateCW => {
            let mut array = buffer.to_array()?;
            array.swap_axes(0, 1);
            array.invert_axis(Axis(1));
            Buffer::from(array)
        }
        FaceRotation::RotateCCW => {
            let mut array = buffer.to_array()?;
            array.swap_axes(0, 1);
            array.invert_axis(Axis(0));
            Buffer::from(array)
        }
        FaceRotation::Backside => unreachable!(),
    })
}

/// The neighbouring face's edge index that physically touches `edge_index` on this face, given
/// the rotation between them. For unrotated neighbours this is the *opposite* edge (my "up"
/// neighbour touches me along *its* "down" edge, not its own "up" edge) - `stitch.rs`'s
/// `neighbour_data` gets this for free because its hardcoded `src_offsets[i]` values already bake
/// in "read the opposite edge" for `Identical`/`ShiftU`/`ShiftV`, and its `(i + 3) % 4`/
/// `(i + 1) % 4` for `RotateCW`/`RotateCCW` select *into that same opposite-biased table* - so
/// composing those with the opposite-edge offset (`+ 2`) gives the actual touching edge here.
fn neighbour_edge_index(edge_index: usize, rotation: &FaceRotation) -> usize {
    match rotation {
        FaceRotation::Identical | FaceRotation::ShiftU | FaceRotation::ShiftV => {
            (edge_index + 2) % EDGE_COUNT
        }
        FaceRotation::RotateCW => (edge_index + 1) % EDGE_COUNT,
        FaceRotation::RotateCCW => (edge_index + 3) % EDGE_COUNT,
        FaceRotation::Backside => unreachable!(),
    }
}

/// Reprojecting each of the 6 cube faces independently (see `reproject.rs`) leaves adjacent
/// faces' real boundary pixels slightly mismatched: GDAL samples each destination pixel at its
/// centre, so the outermost pixel on either side of a shared edge is actually sampled half a
/// pixel inward, on opposite sides of the true edge - two different, nearby points on the
/// (locally varying) source data, bilinearly resampled to two slightly different values. This
/// runs once, directly after `reproject`, on the finest-LOD per-face rasters - before any
/// tiling/downsampling - and averages each pair of adjacent faces' real boundary pixels, writing
/// the reconciled value back into both sides so the mismatch can't propagate into the tile
/// quadtree (downsampling a now-seamless finest level stays seamless at every coarser LOD too).
pub fn reconcile_face_edges<T: Copy + GdalType + NumCast>(
    faces: &HashMap<u32, FaceInfo>,
    _context: &crate::core::dataset::PreprocessContext,
) -> PreprocessResult<()> {
    for face in 0..6u32 {
        for edge_index in 0..EDGE_COUNT {
            let neighbour_face = NEIGHBOURING_FACES[face as usize][edge_index + 1];

            if neighbour_face == face || neighbour_face < face {
                // No crossing (shouldn't happen for a cardinal edge), or already handled from
                // the lower-numbered face's side - each shared edge is only reconciled once.
                continue;
            }

            let Some(face_info) = faces.get(&face) else {
                continue;
            };
            let Some(neighbour_info) = faces.get(&neighbour_face) else {
                continue;
            };

            let dataset = open_face(face_info)?;
            let neighbour_dataset = open_face(neighbour_info)?;

            let (width, height) = dataset.raster_size();
            let (neighbour_width, neighbour_height) = neighbour_dataset.raster_size();

            let rotation = FaceRotation::new(face, neighbour_face);
            let reverse_rotation = FaceRotation::new(neighbour_face, face);
            let neighbour_edge = neighbour_edge_index(edge_index, &rotation);

            let (offset, size) = edge_window(edge_index, width, height);
            let (neighbour_offset, neighbour_size) =
                edge_window(neighbour_edge, neighbour_width, neighbour_height);

            for (raster, neighbour_raster) in izip!(dataset.rasterbands(), neighbour_dataset.rasterbands()) {
                let mut raster = raster?;
                let mut neighbour_raster = neighbour_raster?;

                let own_buffer = raster.read_as::<T>(offset, size, size, None)?;
                let neighbour_buffer =
                    neighbour_raster.read_as::<T>(neighbour_offset, neighbour_size, neighbour_size, None)?;

                let neighbour_aligned = align_to(neighbour_buffer, &rotation)?;

                let own_array = own_buffer.to_array()?;
                let neighbour_array = neighbour_aligned.to_array()?;

                let averaged = Zip::from(&own_array)
                    .and(&neighbour_array)
                    .map_collect(|&a, &b| {
                        let a = a.to_f64().unwrap();
                        let b = b.to_f64().unwrap();
                        T::from((a + b) / 2.0).unwrap()
                    });

                let mut own_write = Buffer::from(averaged.clone());
                raster.write::<T>(offset, size, &mut own_write)?;

                let mut neighbour_write = align_to(Buffer::from(averaged), &reverse_rotation)?;
                neighbour_raster.write::<T>(neighbour_offset, neighbour_size, &mut neighbour_write)?;
            }
        }
    }

    Ok(())
}
