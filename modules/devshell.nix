{inputs, ...}: {
  perSystem = {
    # keep-sorted start
    lib,
    pkgs,
    system,
    # keep-sorted end
    ...
  }: let
    inherit (lib) makeLibraryPath;
    inherit (pkgs) mkShell;

    fenix = inputs.fenix.packages.${system};

    rustToolchain = fenix.combine [
      (fenix.latest.withComponents [
        # keep-sorted start
        "cargo"
        "clippy"
        "rust-src"
        "rust-std"
        "rustc"
        "rustc-codegen-cranelift"
        "rustfmt"
        # keep-sorted end
      ])
      fenix.targets.wasm32-unknown-unknown.latest.rust-std
    ];

    runtimeLibraries = with pkgs; [
      alsa-lib
      libxkbcommon
      udev
      vulkan-loader
      wayland
    ];
  in {
    devShells.default = mkShell {
      buildInputs = runtimeLibraries;

      packages = with pkgs; [
        rustToolchain

        # keep-sorted start
        binaryen
        cargo-modules
        just
        pkg-config
        rustlings
        # keep-sorted end
      ];

      LD_LIBRARY_PATH = makeLibraryPath runtimeLibraries;
    };
  };
}
