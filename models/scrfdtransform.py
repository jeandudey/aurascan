#!/usr/bin/env python3

import argparse

import onnx


def transform(model_path):
    model = onnx.load(model_path)
    graph = model.graph

    for io in list(graph.input) + list(graph.output):
        if io.name == "input.1":
            io.name = "input"

    for node in graph.node:
        node.input[:] = ["input" if x == "input.1" else x for x in node.input]
        node.output[:] = ["output" if x == "output.1" else x for x in node.output]

    for inp in graph.input:
        if inp.name == "input":
            dims = inp.type.tensor_type.shape.dim
            for idx, name in {0: "batch", 2: "height", 3: "width"}.items():
                if name is not None and idx < len(dims):
                    dims[idx].ClearField("dim_value")
                    dims[idx].dim_param = name

    onnx.checker.check_model(model)
    onnx.save(model, model_path)
    print(f"Model saved to {model_path}")


if __name__ == "__main__":
    p = argparse.ArgumentParser(description="Transform the SCRFD ONNX model")
    p.add_argument("model", help="Path to the input ONNX model")
    args = p.parse_args()
    transform(args.model)
