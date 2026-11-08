# Builds Kestrel Engine (a modified aria2) as a static 64-bit Windows
# executable with the Ubuntu mingw-w64 cross compiler.
#
# Derived from Dockerfile.mingw in the aria2 1.37.0 source distribution,
# Copyright (C) Tatsuhiro Tsujikawa, GPL-2.0-or-later.
#
# Usage (from the repository root):
#   docker build -f engine/Dockerfile --target artifacts --output type=local,dest=engine/dist engine
#
# engine/dist then contains kestrel-engine.exe and sources/, which holds
# the complete corresponding source: the patched engine tree, every
# library archive that was linked in, and SHA256SUMS.

FROM ubuntu:22.04 AS build

ARG HOST=x86_64-w64-mingw32
ARG ARIA2_VERSION=1.37.0
ARG ARIA2_SHA256=60a420ad7085eb616cb6e2bdf0a7206d68ff3d37fb5a956dc44242eb2f79b66b
ARG ENGINE_VERSION=1.37.0-k1
ARG GMP_VERSION=6.3.0
ARG EXPAT_VERSION=2.7.1
ARG SQLITE_VERSION=3460100
ARG SQLITE_YEAR=2024
ARG ZLIB_VERSION=1.3.1
ARG CARES_VERSION=1.34.5
ARG LIBSSH2_VERSION=1.11.1

RUN apt-get update && \
    DEBIAN_FRONTEND="noninteractive" \
    apt-get install -y --no-install-recommends \
        make binutils autoconf automake autotools-dev libtool \
        patch ca-certificates xz-utils bzip2 \
        pkg-config git curl dpkg-dev gcc-mingw-w64 g++-mingw-w64 \
        autopoint libcppunit-dev libxml2-dev libgcrypt20-dev lzip \
        python3-docutils

WORKDIR /src
RUN curl -fsSL -O https://github.com/aria2/aria2/releases/download/release-${ARIA2_VERSION}/aria2-${ARIA2_VERSION}.tar.xz && \
    echo "${ARIA2_SHA256}  aria2-${ARIA2_VERSION}.tar.xz" | sha256sum -c - && \
    curl -fsSL -O https://gmplib.org/download/gmp/gmp-${GMP_VERSION}.tar.xz && \
    curl -fsSL -O https://github.com/libexpat/libexpat/releases/download/R_$(echo ${EXPAT_VERSION} | tr . _)/expat-${EXPAT_VERSION}.tar.bz2 && \
    curl -fsSL -O https://www.sqlite.org/${SQLITE_YEAR}/sqlite-autoconf-${SQLITE_VERSION}.tar.gz && \
    curl -fsSL -O https://github.com/madler/zlib/releases/download/v${ZLIB_VERSION}/zlib-${ZLIB_VERSION}.tar.gz && \
    curl -fsSL -O https://github.com/c-ares/c-ares/releases/download/v${CARES_VERSION}/c-ares-${CARES_VERSION}.tar.gz && \
    curl -fsSL -O https://libssh2.org/download/libssh2-${LIBSSH2_VERSION}.tar.gz

# Pinned checksums. Update them together with the version arguments above.
RUN printf '%s\n' \
        "a3c2b80201b89e68616f4ad30bc66aee4927c3ce50e33929ca819d5c43538898  gmp-6.3.0.tar.xz" \
        "45c98ae1e9b5127325d25186cf8c511fa814078e9efeae7987a574b482b79b3d  expat-2.7.1.tar.bz2" \
        "67d3fe6d268e6eaddcae3727fce58fcc8e9c53869bdd07a0c61e38ddf2965071  sqlite-autoconf-3460100.tar.gz" \
        "9a93b2b7dfdac77ceba5a558a580e74667dd6fede4585b91eefb60f03b72df23  zlib-1.3.1.tar.gz" \
        "7d935790e9af081c25c495fd13c2cfcda4792983418e96358ef6e7320ee06346  c-ares-1.34.5.tar.gz" \
        "d9ec76cbe34db98eec3539fe2c899d26b0c837cb3eb466a56b0f109cabf658f7  libssh2-1.11.1.tar.gz" \
    | sha256sum -c -

WORKDIR /build
RUN tar xf /src/gmp-${GMP_VERSION}.tar.xz && \
    cd gmp-${GMP_VERSION} && \
    ./configure \
        --disable-shared \
        --enable-static \
        --prefix=/usr/local/$HOST \
        --host=$HOST \
        --disable-cxx \
        --enable-fat \
        CFLAGS="-mtune=generic -O2 -g0" && \
    make -j$(nproc) install

RUN tar xf /src/expat-${EXPAT_VERSION}.tar.bz2 && \
    cd expat-${EXPAT_VERSION} && \
    ./configure \
        --disable-shared \
        --enable-static \
        --without-docbook \
        --without-examples \
        --without-tests \
        --prefix=/usr/local/$HOST \
        --host=$HOST \
        --build=`dpkg-architecture -qDEB_BUILD_GNU_TYPE` && \
    make -j$(nproc) install

RUN tar xf /src/sqlite-autoconf-${SQLITE_VERSION}.tar.gz && \
    cd sqlite-autoconf-${SQLITE_VERSION} && \
    ./configure \
        --disable-shared \
        --enable-static \
        --prefix=/usr/local/$HOST \
        --host=$HOST \
        --build=`dpkg-architecture -qDEB_BUILD_GNU_TYPE` && \
    make -j$(nproc) install

RUN tar xf /src/zlib-${ZLIB_VERSION}.tar.gz && \
    cd zlib-${ZLIB_VERSION} && \
    CC=$HOST-gcc \
    AR=$HOST-ar \
    LD=$HOST-ld \
    RANLIB=$HOST-ranlib \
    STRIP=$HOST-strip \
    ./configure \
        --prefix=/usr/local/$HOST \
        --libdir=/usr/local/$HOST/lib \
        --includedir=/usr/local/$HOST/include \
        --static && \
    make -j$(nproc) install

RUN tar xf /src/c-ares-${CARES_VERSION}.tar.gz && \
    cd c-ares-${CARES_VERSION} && \
    ./configure \
        --disable-shared \
        --enable-static \
        --disable-tests \
        --prefix=/usr/local/$HOST \
        --host=$HOST \
        --build=`dpkg-architecture -qDEB_BUILD_GNU_TYPE` \
        LIBS="-lws2_32" && \
    make -j$(nproc) install

RUN tar xf /src/libssh2-${LIBSSH2_VERSION}.tar.gz && \
    cd libssh2-${LIBSSH2_VERSION} && \
    ./configure \
        --disable-shared \
        --enable-static \
        --disable-examples-build \
        --disable-tests \
        --prefix=/usr/local/$HOST \
        --host=$HOST \
        --build=`dpkg-architecture -qDEB_BUILD_GNU_TYPE` \
        LIBS="-lws2_32" && \
    make -j$(nproc) install

COPY patches/ /patches/
RUN tar xf /src/aria2-${ARIA2_VERSION}.tar.xz && \
    mv aria2-${ARIA2_VERSION} kestrel-engine-${ENGINE_VERSION} && \
    cd kestrel-engine-${ENGINE_VERSION} && \
    for p in /patches/*.patch; do patch -p1 --forward < "$p"; done && \
    cd .. && \
    tar cJf /src/kestrel-engine-${ENGINE_VERSION}.tar.xz kestrel-engine-${ENGINE_VERSION}

RUN cd kestrel-engine-${ENGINE_VERSION} && \
    autoreconf -i && \
    HOST=$HOST ./mingw-config && \
    make -j$(nproc) && \
    $HOST-strip src/aria2c.exe && \
    mkdir -p /out/sources && \
    cp src/aria2c.exe /out/kestrel-engine.exe && \
    cp /src/*.tar.* /out/sources/ && \
    cp /patches/*.patch /out/sources/ && \
    cd /out/sources && sha256sum * > SHA256SUMS && \
    cd /out && sha256sum kestrel-engine.exe > kestrel-engine.exe.sha256

FROM scratch AS artifacts
COPY --from=build /out/ /
