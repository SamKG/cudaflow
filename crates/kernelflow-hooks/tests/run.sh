#!/bin/bash
set -o errexit
set -o pipefail
set -o nounset
set -o xtrace

function run() {
    # ./build.sh
    #

    export SO_DIR="/scratch/gpfs/samyakg/Research/reaper/reaper/target/debug"
    export BASE_IMAGE="cuda_12.9.0-cudnn-devel-ubi9.sif"

    singularity exec --nv \
        -B "${SO_DIR}/libcuda.so:/libcuda.so:ro" \
        -B "${SO_DIR}/libcuda.so.1:/libcuda.so.1:ro" \
        -B "${SO_DIR}/libcudart.so:/libcudart.so:ro" \
        -B "${SO_DIR}/libcudart.so.1:/libcudart.so.1:ro" \
        -B "${SO_DIR}/ld.so.preload:/etc/ld.so.preload:ro" \
        /scratch/gpfs/samyakg/containers/${BASE_IMAGE} bash -c "./kernel"
}

run
