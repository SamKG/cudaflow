#!/bin/bash
module load gcc-toolset/14
export PATH=$PATH:/usr/local/cuda/bin
which nvcc
#make clean
make
