#!/bin/bash

# Used to prefix a binary message with its length as four bytes
length_prefix() {
    local message="$1"
    local hex_length=$(printf "%08x" ${#message})
    (echo "$hex_length" | xxd -r -p; echo -n "$message")
}