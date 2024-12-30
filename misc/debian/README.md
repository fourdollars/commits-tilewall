# debian

Incomplete Debian packaging merely for building the build and runtime dependencies packages for clean installation and removal.

## Prerequisites

The following packages must be installed to build the dependency packages:

* devscripts
* equivs

## Building the build dependency package

1. Launch your preferred terminal.
1. Change the working directory to _the parent directory_ of the one hosting this document.
1. Run the following command to build the build dependency package:

    ```bash
    mk-build-deps
    ```

   The built build dependency package will be available in the working directory, installing it by running the following command:

    ```bash
    sudo apt install ./commits-tilewall-build-deps_1.0_all.deb
    ```

## Building the runtime dependency package

1. Launch your preferred terminal.
1. Change the working directory to _the parent directory_ of the one hosting this document.
1. Run the following command to build the build dependency package:

    ```bash
    equivs-build debian/control
    ```

   The built runtime dependency package will be available in the working directory, installing it by running the following command:

    ```bash
    sudo apt install ./commits-tilewall-deps_1.0_all.deb
    ```
