use bevy::shader::ShaderDefVal;

use crate::debug::DebugTerrain;

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct TilingPrepassPipelineKey: u32 {
        const NONE           = 0;
        const REFINE_TILES   = 1 <<  0;
        const PREPARE_ROOT   = 1 <<  1;
        const PREPARE_NEXT   = 1 <<  2;
        const PREPARE_RENDER = 1 <<  3;
        const SPHERICAL      = 1 <<  4;
        const HIGH_PRECISION = 1 <<  5;
        const MORPH          = 1 <<  6;
        const BLEND          = 1 <<  7;
    }
}

impl TilingPrepassPipelineKey {
    pub fn from_debug(debug: &DebugTerrain) -> Self {
        let mut key = TilingPrepassPipelineKey::NONE;

        if debug.high_precision {
            key |= TilingPrepassPipelineKey::HIGH_PRECISION;
        }
        if debug.morph {
            key |= TilingPrepassPipelineKey::MORPH;
        }
        if debug.blend {
            key |= TilingPrepassPipelineKey::BLEND;
        }

        key
    }

    pub fn shader_defs(&self) -> Vec<ShaderDefVal> {
        let mut shader_defs = Vec::new();

        shader_defs.push("PREPASS".into());

        if self.contains(TilingPrepassPipelineKey::SPHERICAL) {
            shader_defs.push("SPHERICAL".into());
        }
        if self.contains(TilingPrepassPipelineKey::HIGH_PRECISION) {
            shader_defs.push("HIGH_PRECISION".into());
        }
        if self.contains(TilingPrepassPipelineKey::MORPH) {
            shader_defs.push("MORPH".into());
        }
        if self.contains(TilingPrepassPipelineKey::BLEND) {
            shader_defs.push("BLEND".into());
        }

        shader_defs
    }
}
