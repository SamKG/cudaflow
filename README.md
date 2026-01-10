# cudaflow

This repo demonstrates how to build hooks for CUDA Driver APIs, using Rust.

# What's a hook?

Sometimes, it's useful to write your own logic for CUDA driver calls. For example, I may want to keep a record every time a kernel is launched using CUDA. By writing my own hook for cuLaunchKernel, I can then add my own logic that tracks the launches.

# Examples
See: [examples/cuda-init-hook] for an example of how to use the crates in this repo.

More docs coming soon!
