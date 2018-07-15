#/bin/bash

function buildlibsodium {
  local scriptdir="$( cd "$( dirname "$BASH_SOURCE[0]" )" && pwd )"
  local basedir="${scriptdir}/third-party/libsodium"
  local builddir="${basedir}/build"
  mkdir -p "${builddir}"

  if [ ! -f "${basedir}/configure" ]; then
    (cd "${basedir}" && ./autogen.sh)
    if [ $? -ne 0 ]; then
      exit 1
    fi
  fi

  if [ ! -f "${basedir}/Makefile" ]; then
    (cd "${basedir}" && ./configure --prefix="${builddir}" --disable-shared --enable-static --with-pthreads --with-pic)
    if [ $? -ne 0 ]; then
      exit 1
    fi
  fi

  if [ ! -f "${builddir}/lib/libsodium.a" ]; then
    (cd "${basedir}" && make -j5 && make install)
    if [ $? -ne 0 ]; then
      exit 1
    fi
  fi

  export SODIUM_LIB_DIR="${builddir}/lib"
  export SODIUM_INC_DIR="${builddir}/include"
  export SODIUM_STATIC="1"
}

git submodule update --init --recursive
buildlibsodium
