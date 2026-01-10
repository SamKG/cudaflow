CC      = nvcc
CFLAGS  = -O3 -arch=sm_90 -std=c++20 -g --cudart=shared
LDFLAGS = -lcublasLt -lcuda -lcublas -I$(CUDA_HOME)/targets/x86_64-linux/include -L$(CUDA_HOME)/targets/x86_64-linux/lib

TARGET  = kernel
SRCS    = kernel.cu
OBJS    = $(SRCS:.cu=.o)

all: $(TARGET)

# Linking step: combine the object files into the final executable.
$(TARGET): $(OBJS)
	$(CC) $(CFLAGS) $(OBJS) -o $(TARGET) $(LDFLAGS)

# Compilation step: compile the CUDA source file into an object file.
%.o: %.cu
	$(CC) $(CFLAGS) $(LDFLAGS) -c $< -o $@

clean:
	rm -f $(OBJS) $(TARGET)
