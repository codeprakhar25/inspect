#!/usr/bin/env zsh
# inspect shell widget for zsh
# Bound to Ctrl-] — explains the current zle buffer inline.

_inspect_widget() {
    local cmd="$BUFFER"
    if [ -n "$cmd" ]; then
        zle -I
        echo ""
        inspect "$cmd"
    fi
    zle reset-prompt
}

zle -N _inspect_widget
bindkey "^]" _inspect_widget
