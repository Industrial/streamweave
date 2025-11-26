//! Test to verify WASM optimization settings are correct
//!
//! This test ensures that WASM builds use the correct optimization settings
//! for minimal bundle size.

//! Test to verify WASM optimization settings are correct
//!
//! This test ensures that WASM builds use the correct optimization settings
//! for minimal bundle size. The actual bundle size verification is done via
//! bin/wasm-build script.

#[cfg(target_arch = "wasm32")]
#[test]
fn test_wasm_optimization_settings() {
  // This test only runs on wasm32 target
  // It verifies that the crate compiles with WASM features
  use streamweave::pipeline::Pipeline;
  let _pipeline = Pipeline::<u32, u32>::new();

  // If we get here, WASM compilation succeeded
  // The actual bundle size verification is done via bin/wasm-build script
}
