use burn_onnx::{LoadStrategy, ModelGen};

fn main() {
    ModelGen::new()
        .input("model/scrfd_500m.onnx")
        .out_dir("scrfd_500m/")
        .load_strategy(LoadStrategy::Embedded)
        .run_from_script();
}
