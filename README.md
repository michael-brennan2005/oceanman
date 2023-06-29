# OceanMan

OceanMan is a real-time renderer focused on making realism. It is developed using Rust, WebGPU, and wgpu. It is currently in version 0.2.

## Installation.

OceanMan was most recently developed using rustc 1.70.0 (90c541806 2023-05-31).

```bash
git clone https://github.com/tech0tron/oceanman
cargo build
```

## Usage

OceanMan requires a json file to render. The json file describes how the scene should be setup, and an example one is located in the resources/ directory.
```bash
oceanman scene.json
```

## Features

* Phong shading
* Texturing
* Shadow mapping
* Configurable models and model positions/orientations.
* One directional light (configurable)
* Loading in .obj files

## Images

## License

[MIT](https://choosealicense.com/licenses/mit/)