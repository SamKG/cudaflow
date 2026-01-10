// test_malloc_hook.cu
#include <chrono>
#include <cuda_runtime.h>
#include <cstdio>
#include <cuda.h>
#include <thread>

__global__ void nopKernel(int *data, int *data_2, int tmp, int tmp2) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    data[idx] = idx;            // write something so the memory gets touched
}

int main() {
    printf("Starting CUDA malloc hook test...\n");
    printf("Post cuInit!\n");
    const int N = 256;
    int *d_data = nullptr;

    cudaError_t err = cudaMalloc(reinterpret_cast<void **>(&d_data),
                                 N * sizeof(int));
    if (err != cudaSuccess) {
        std::fprintf(stderr, "cudaMalloc failed: %s\n",
                     cudaGetErrorString(err));
        return 1;
    }
    int *d_data_2 = nullptr;

    const int M = 1024;
    err = cudaMalloc(reinterpret_cast<void **>(&d_data_2),
                                 M * sizeof(int));
    if (err != cudaSuccess) {
        std::fprintf(stderr, "cudaMalloc failed: %s\n",
                     cudaGetErrorString(err));
        return 1;
    }

    // Launch a trivial kernel
    for (int i = 0; i < 1000000000; i++) {
        int a = 10;
        nopKernel<<<1, N>>>(d_data, d_data_2, a, a);
        // printf("Kernel launched successfully!\n");
        err = cudaDeviceSynchronize();
        if (err != cudaSuccess) {
            std::fprintf(stderr, "kernel or sync failed: %s\n",
                         cudaGetErrorString(err));
        }
        // sleep for a second
        // std::this_thread::sleep_for(std::chrono::milliseconds(1000));
    }

    cudaFree(d_data);
    printf("Memory freed successfully!\n");
    return 0;
}
