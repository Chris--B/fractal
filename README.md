# Fractals are cool

This is a repo where I experiment with rendering fractals. Currently, there are two binaries that you can run:
1. [`gen`](src/gen.rs)
2. [`view`](src/view.rs)

Everything is currently CPU-driven, but can be accelerated using `rayon` by building with the `rayon` [Cargo feature](https://doc.rust-lang.org/cargo/reference/features.html) enabled.

```
$ cargo run --all-features
```

## Gen

This runs offline and attempts to create a single, high-quality image that is then saved to disk.

## View

This renders mandelbrot with colors and iteratively updates it with more iterations of `z = z^2 + c`. Consult the source code for the most up-to-date list of controls.

![Example run of View](https://user-images.githubusercontent.com/1052157/111042554-ea93e600-840b-11eb-9c96-d7c006525425.png)
