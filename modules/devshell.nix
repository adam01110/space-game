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

    cargoCrap = pkgs.rustPlatform.buildRustPackage rec {
      pname = "cargo-crap";
      version = "0.5.0";

      src = pkgs.fetchCrate {
        inherit pname version;
        hash = "sha256-5RhRFUh1w5/yItkmc3Vk1B6oyrmzKKl6EEZ3v0aBLwk=";
      };

      cargoHash = "sha256-vPdzZIeXsjICz5icPrr2LQ4GrcMSZe2nRa65iyzLH7Q=";

      # The published crate omits workspace fixtures required by its tests.
      doCheck = false;
    };

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
        cargo-mutants
        cargoCrap
        just
        pkg-config
        # keep-sorted end
      ];

      LD_LIBRARY_PATH = makeLibraryPath runtimeLibraries;
    };
  };
}
