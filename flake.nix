{
  description = "pile, the binary of multipile: the toolchain it is built, checked and released with";

  # Every tool a script here runs comes from these inputs, pinned by
  # flake.lock. Rust is an exact version, never latest. The rest are the
  # versions nixpkgs holds at the pinned commit, and zig is named by its
  # version, because nixpkgs' default zig moves.
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      crane,
    }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      forAll =
        f:
        nixpkgs.lib.genAttrs systems (
          system:
          let
            pkgs = import nixpkgs {
              inherit system;
              overlays = [ rust-overlay.overlays.default ];
            };
            # The targets are every platform a release is built for
            # (scripts/cross). Linux is built on musl, so the binary links no
            # libc.
            toolchain = pkgs.rust-bin.stable."1.99.1".default.override {
              extensions = [
                "clippy"
                "rustfmt"
              ];
              targets = [
                "aarch64-unknown-linux-musl"
                "x86_64-unknown-linux-musl"
                "aarch64-apple-darwin"
                "x86_64-apple-darwin"
              ];
            };
            zig = pkgs.zig_0_16;
            # nixpkgs wraps cargo-zigbuild with its own zig. This is the same
            # zig as the shell's, so every link uses one zig.
            cargo-zigbuild = pkgs.cargo-zigbuild.override { inherit zig; };
            craneLib = (crane.mkLib pkgs).overrideToolchain toolchain;
            src = pkgs.lib.cleanSourceWith {
              src = ./.;
              # More than Cargo's own files: the guide's pages are Markdown,
              # which the crates read when they are built, and the tests
              # read their fixtures, piles among them.
              filter =
                path: type:
                (craneLib.filterCargoSources path type)
                || pkgs.lib.hasSuffix ".md" path
                || pkgs.lib.hasInfix "/tests/fixtures" path
                || pkgs.lib.hasInfix "/.pile" path;
            };
            common = {
              inherit src;
              strictDeps = true;
              pname = "pile";
              # The tests drive the built binary against real git
              # repositories.
              nativeCheckInputs = [ pkgs.git ];
            };
            cargoArtifacts = craneLib.buildDepsOnly common;
          in
          f {
            inherit
              pkgs
              toolchain
              zig
              cargo-zigbuild
              craneLib
              common
              cargoArtifacts
              ;
          }
        );
    in
    {
      packages = forAll (
        { craneLib, common, cargoArtifacts, ... }:
        {
          # pile, built for this machine. Its tests run in checks.test, not
          # here. A release is built by scripts/release, not by this.
          default = craneLib.buildPackage (
            common
            // {
              inherit cargoArtifacts;
              cargoExtraArgs = "--locked -p pile";
              doCheck = false;
            }
          );
        }
      );

      # What `nix flake check` runs: the tests, clippy and rustfmt. crane
      # passes --locked to each. scripts/check runs these and more: the
      # journeys, the cross builds and doctor, which need the shell.
      checks = forAll (
        { craneLib, common, cargoArtifacts, ... }:
        {
          test = craneLib.cargoTest (
            common
            // {
              inherit cargoArtifacts;
              cargoTestExtraArgs = "--workspace";
            }
          );
          clippy = craneLib.cargoClippy (
            common
            // {
              inherit cargoArtifacts;
              cargoClippyExtraArgs = "--workspace --all-targets -- --deny warnings";
            }
          );
          fmt = craneLib.cargoFmt { inherit (common) src pname; };
        }
      );

      # The shell every script here runs in: `nix develop`, or
      # `nix develop -c <script>`.
      devShells = forAll (
        {
          pkgs,
          toolchain,
          zig,
          cargo-zigbuild,
          ...
        }:
        {
          default = pkgs.mkShell {
            packages = [
              toolchain
              zig
              cargo-zigbuild
              pkgs.jq
              pkgs.asciinema
              pkgs.git
              pkgs.curl
            ];
            # bin/ is first on PATH, so `pile` is what scripts/build last
            # wrote. On Linux, a build for this machine links with zig
            # (scripts/zig-cc), not with whatever cc the machine has.
            shellHook = ''
              root=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
              export PATH="$root/bin:$PATH"
              export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER="$root/scripts/zig-cc"
              export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER="$root/scripts/zig-cc"
            '';
          };
        }
      );

      formatter = forAll ({ pkgs, ... }: pkgs.nixfmt);
    };
}
