#!/bin/sh
# Records a person playing Brickspire for the catalog preview: run it in a
# terminal window of 100x30, play, and quit with q when you are done (20
# to 60 seconds is the preview's length; it can be trimmed after). The
# game runs with the terminal identity hub/RECORDING.md specifies, in a
# throwaway data folder and as USER=player, so no real name or best score
# shows. Sound plays as usual; the recording has none.
#   sh asciiverse/record-play.sh [OUT.cast]
set -eu
cd "$(dirname "$0")/.."
out=${1:-$(pwd)/asciiverse/brickspire-played.cast}
[ -x target/release/brickspire ] || cargo build --release
data=$(mktemp -d)
trap 'rm -rf "$data"' EXIT
cols=$(tput cols 2>/dev/null || echo 0)
rows=$(tput lines 2>/dev/null || echo 0)
if [ "$cols" -ne 100 ] || [ "$rows" -ne 30 ]; then
  echo "This window is ${cols}x${rows}; 100x30 makes what you see match the recording."
  printf "Record anyway? [y/N] "
  read -r a
  [ "$a" = y ] || exit 1
fi
rm -f "$out"
env -u VTE_VERSION -u TERM_PROGRAM -u TERM_PROGRAM_VERSION -u KITTY_WINDOW_ID \
  -u WEZTERM_PANE -u KONSOLE_VERSION -u ALACRITTY_WINDOW_ID -u WT_SESSION \
  TERM=xterm-256color COLORTERM=truecolor LANG=C.UTF-8 LC_ALL=C.UTF-8 \
  USER=player LOGNAME=player XDG_DATA_HOME="$data" XDG_STATE_HOME="$data" \
  PATH="$(pwd)/target/release:$PATH" \
  asciinema rec --window-size 100x30 --idle-time-limit 2 \
    --capture-env TERM,COLORTERM,LANG --title Brickspire \
    --command brickspire "$out"   # the name alone: the command is kept in the cast
echo "Recorded $out ($(du -h "$out" | cut -f1))"
