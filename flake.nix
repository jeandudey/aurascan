{
  description = "Dev shell with buck2";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };

        runtimeLibs = [
          pkgs.libx11
          pkgs.libxkbcommon
          pkgs.mesa
          pkgs.opencv
          pkgs.vulkan-loader
          pkgs.wayland
        ];
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = [
            pkgs.opencv
          ];

          packages = [
            pkgs.buck2
            (pkgs.clangStdenv.cc)
            pkgs.lld
            pkgs.pkg-config
            pkgs.reindeer
          ];

          LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath runtimeLibs;
        };
      });
}
