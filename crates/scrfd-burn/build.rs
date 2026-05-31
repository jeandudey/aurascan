use burn_onnx::{LoadStrategy, ModelGen};
use std::env;

const MODELS: &[&str] = &[
    "1g",
    "2.5g",
    "2.5g_bnkps",
    "10g",
    "10g_bnkps",
    "34g",
    "500m",
    "500m_bnkps",
];

fn main() {
    let load_strategy = if env::var("CARGO_FEATURE_EMBEDDED").is_ok() {
        LoadStrategy::Embedded
    } else {
        LoadStrategy::None
    };

    for model in MODELS {
        ModelGen::new()
            .input(&format!("model/scrfd_{}.onnx", model))
            .out_dir(&format!("scrfd_{}/", model))
            .load_strategy(load_strategy)
            .run_from_script();
    }
}
