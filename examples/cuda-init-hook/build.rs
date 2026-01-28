fn main() -> anyhow::Result<()> {
    cuda_interposer_build::InterposerBuilder::new().build()
}
