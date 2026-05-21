#!/usr/bin/env bash
# inspect shell widget for bash
# Bound to Ctrl-] — explains the current readline buffer inline.

_inspect_widget() {
    local cmd="$READLINE_LINE"
    if [ -n "$cmd" ]; then
        echo ""
        inspect "$cmd"
    fi
}

bind -x '"\C-]": _inspect_widget'
