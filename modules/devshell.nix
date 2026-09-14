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

    wasmBindgenCli = pkgs.buildWasmBindgenCli rec {
      src = pkgs.fetchCrate {
        pname = "wasm-bindgen-cli";
        version = "0.2.127";
        hash = "sha256-di+qBAdd7pENLiIB9CoZoab+W5xeDoByMREcCGTSzWo=";
      };

      cargoDeps = pkgs.rustPlatform.fetchCargoVendor {
        inherit src;
        inherit (src) pname version;
        hash = "sha256-FTv2GZIAQs0ePdIZXIXil7JbZ6kIT05VG6vqC1qNFxQ=";
      };
    };
  in {
    devShells.default = mkShell {
      buildInputs = runtimeLibraries;

      packages = with pkgs; [
        rustToolchain
        caddy
        wasmBindgenCli

        # keep-sorted start
        binaryen
        cargo-audit
        cargo-modules
        just
        pkg-config
        # keep-sorted end
      ];

      LD_LIBRARY_PATH = makeLibraryPath runtimeLibraries;
    };
  };
}
