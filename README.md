# PathTracer: 3D Scene Rendering Application
PathTracer is a robust and high-performance 3D rendering application, capable of rendering intricate scenes containing basic geometric shapes, complex mathematical 3D object functions (like mandelbulb, julia fract, torus and mebius strip), and OBJ files with 3D models. The application generates near real-world images, based on the Path Tracing algorithm.

It leverages the computing power of GPU using the OpenCL runtime, thus ensuring quick and efficient rendering.

### Requirements
Ensure that your system meets the following requirements:
- OpenCL version 1.2 or later
- GPU device available

### Building and Running the Application
To build and run PathTracer, follow the steps below:
1. Open Terminal and navigate to the root directory of the repository.
2. Run the make command to build the application:
```sh
$> make
```
3. After the successful build, run the PathTracer application with your desired scene file:
```sh
$> ./build/PathTracer <your-scene-file.sc>
```
*Note:* Replace _<your-scene-file.sc>_ with the name of your actual scene file.

### Example Scenes
The repository contains a set of example scenes that you can use to evaluate the capabilities of PathTracer. You can find these scenes in the `resources/scenes` directory.

### Contributing
We welcome contributions! If you find a bug or would like to add a new feature, feel free to create an issue or open a pull request.

### License
PathTracer is licensed under the [GPL-3.0 license](./LICENSE).

