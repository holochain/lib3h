#!/bin/bash

# usage:
# ```
# source ./capnp-build-and-source.bash
# which capnp
# capnp --version
# ````

# run the main code in a function so we can easily capture exits
function __sub() {
  local work_dir=""
  local src_dir="${BASH_SOURCE[0]}"
  while [ -h "${src_dir}" ]; do
    local work_dir="$(cd -P "$(dirname "${src_dir}")" >/dev/null 2>&1 && pwd)"
    local src_dir="$(readlink "${src_dir}")"
    [[ ${src_dir} != /* ]] && local src_dir="${work_dir}/${src_dir}"
  done
  local work_dir="$(cd -P "$(dirname "${src_dir}")" >/dev/null 2>&1 && pwd)"

  cd "${work_dir}"

  mkdir -p ".local-capnproto-binary"
  cd ".local-capnproto-binary"

  local name="capnproto-v0.7.0.tar.gz"

  if [ ! -f $name ]; then
    curl -L -o $name https://github.com/capnproto/capnproto/archive/v0.7.0.tar.gz
    if [ $? -ne 0 ]; then
      echo "failed to download capnp"
      exit 1
    fi
  fi

  if [ ! -d capnproto-0.7.0 ]; then
    tar xf $name
    if [ $? -ne 0 ]; then
      echo "failed to extract capnp"
      exit 1
    fi
  fi

  cd capnproto-0.7.0/c++

  if [ ! -f capnp ]; then
    autoreconf -i && ./configure && make -j$(numproc)
    if [ $? -ne 0 ]; then
      echo "failed to build capnp"
      exit 1
    fi
  fi

  echo "export PATH=\"\$PATH:$(pwd)\""
  export PATH="$PATH:$(pwd)"
}

# make sure we go back to our original dir, even if sourced
function __main() {
  local startdir="$(pwd)"
  __sub || true
  cd "$startdir"
}

# execute the main function and unset fns to not polute sourced shell
__main
unset __main
unset __sub
