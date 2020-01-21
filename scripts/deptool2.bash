#!/bin/bash
set +e
CRATE=$1
REPO=$2
BRANCH=$3
ROOT=`pwd`
cd $ROOT/crates
dirs=`ls`
for d in $dirs
do 
    cd $d 
    echo Processing crate from directory \"$d\"...
    cargo-add add $CRATE --git $REPO --branch $BRANCH
    cd .. 
done
cd $ROOT
