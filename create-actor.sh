#!/bin/bash

if [ -n "$1" ]; then
    mkdir actors/$1
    mkdir tests/$1
    cp -r templates/actor/* actors/$1
    cp -r templates/test/* tests/$1
    sed -i "s/<actor-name>/$1/g" actors/$1/Cargo.toml
    sed -i "s/<actor-name>/$1/g" tests/$1/Cargo.toml
else
    echo "Please provide a name for the actor."
    exit 1
fi