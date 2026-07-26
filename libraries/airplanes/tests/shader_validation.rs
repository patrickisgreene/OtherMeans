use naga::valid::{Capabilities, ValidationFlags, Validator};

/// Both shaders under test use bevy's `naga_oil` `#import` preprocessor syntax, which isn't valid
/// raw WGSL - plain `naga` (used here to stay independent of bevy's shader-composition machinery)
/// can't parse it directly. Since the imports only ever bring in `view.world_position` and the
/// `position_world_to_clip` function, both used exactly as bevy_pbr defines them, stubbing those
/// two symbols in place of the `#import` lines lets the rest of the file - all the actual
/// hand-written/adapted logic - get parsed and validated for real.
fn strip_imports_and_stub(source: &str) -> String {
    let stub = "struct ViewStub { world_position: vec3<f32> }\n\
                var<private> view: ViewStub;\n\
                fn position_world_to_clip(world_position: vec3<f32>) -> vec4<f32> {\n\
                    return vec4<f32>(world_position, 1.0);\n\
                }\n";
    let body: String = source
        .lines()
        .filter(|line| !line.trim_start().starts_with("#import"))
        .collect::<Vec<_>>()
        .join("\n");
    format!("{stub}{body}")
}

/// Parses and fully validates a WGSL shader with `naga` - the same frontend/validator wgpu uses
/// internally when compiling a pipeline. A shader that fails here would make the pipeline
/// silently fail to specialize rather than crash the whole app, which could look exactly like
/// "the batch entities exist but nothing ever draws" - the symptom under investigation - without
/// ever showing up as a panic in a log.
fn validate(label: &str, source: &str) {
    let source = strip_imports_and_stub(source);
    let module = naga::front::wgsl::parse_str(&source)
        .unwrap_or_else(|e| panic!("{label}: WGSL parse error:\n{}", e.emit_to_string(&source)));

    let mut validator = Validator::new(ValidationFlags::all(), Capabilities::all());
    validator
        .validate(&module)
        .unwrap_or_else(|e| panic!("{label}: WGSL validation error:\n{e}"));
}

/// Regression test for the reported "AirplaneTile batches exist but nothing ever renders" bug:
/// confirms `airplanes.wgsl` (adapted from `shipping.wgsl` by removing the ground-rest altitude
/// lift - see `assets/shaders/airplanes.wgsl`'s doc comment) is not itself the problem.
#[test]
fn airplanes_shader_is_valid() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/shaders/airplanes.wgsl"
    );
    let source = std::fs::read_to_string(path).expect("read airplanes.wgsl");
    validate("airplanes.wgsl", &source);
}

/// Control: the known-working shipping shader must also pass under the exact same
/// parse+validate call, so a failure on `airplanes.wgsl` above can't be blamed on the test
/// harness itself (wrong naga version/capabilities/flags).
#[test]
fn shipping_shader_is_valid_for_comparison() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/shaders/shipping.wgsl"
    );
    let source = std::fs::read_to_string(path).expect("read shipping.wgsl");
    validate("shipping.wgsl", &source);
}
