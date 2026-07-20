use std::ops::Range;

use bevy::{
    ecs::entity::EntityHash,
    material::{descriptor::CachedRenderPipelineId, labels::DrawFunctionId},
    prelude::*,
    render::{
        render_phase::{
            CachedRenderPipelinePhaseItem, PhaseItem, PhaseItemExtraIndex, SortedPhaseItem,
        },
        sync_world::MainEntity,
        view::ExtractedView,
    },
};
use indexmap::IndexMap;

pub struct TerrainItem {
    pub representative_entity: (Entity, MainEntity),
    pub draw_function: DrawFunctionId,
    pub pipeline: CachedRenderPipelineId,
    pub batch_range: Range<u32>,
    pub extra_index: PhaseItemExtraIndex,
    pub order: u32,
}

impl PhaseItem for TerrainItem {
    const AUTOMATIC_BATCHING: bool = false;

    #[inline]
    fn entity(&self) -> Entity {
        self.representative_entity.0
    }

    #[inline]
    fn main_entity(&self) -> MainEntity {
        self.representative_entity.1
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.draw_function
    }

    #[inline]
    fn batch_range(&self) -> &Range<u32> {
        &self.batch_range
    }

    fn batch_range_mut(&mut self) -> &mut Range<u32> {
        &mut self.batch_range
    }

    fn extra_index(&self) -> PhaseItemExtraIndex {
        self.extra_index.clone()
    }

    fn batch_range_and_extra_index_mut(&mut self) -> (&mut Range<u32>, &mut PhaseItemExtraIndex) {
        (&mut self.batch_range, &mut self.extra_index)
    }
}

impl SortedPhaseItem for TerrainItem {
    type SortKey = u32;

    fn sort_key(&self) -> Self::SortKey {
        u32::MAX - self.order
    }

    fn recalculate_sort_keys(
        _items: &mut IndexMap<(Entity, MainEntity), Self, EntityHash>,
        _view: &ExtractedView,
    ) {
        // The sort key is derived from `order`, which is fixed at insertion time and
        // does not depend on the view, so there is nothing to recalculate here.
    }

    fn indexed(&self) -> bool {
        false
    }
}

impl CachedRenderPipelinePhaseItem for TerrainItem {
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.pipeline
    }
}
