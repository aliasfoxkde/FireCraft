#!/usr/bin/env bash
# Wait for local cargo admission (no competing compiler processes and
# host PSI under the gate), then run the engine history-rewind test.
set -u
cd /nas/Temp/repos/FireCraft/.claude/worktrees/wt-hist-session || exit 1

while true; do
  compilers=$(ps -eo args | grep -E '(cargo-clippy|/cargo (build|check|clippy|test|run)|/rustc --crate-name)' | grep -v 'grep -E' | wc -l)
  cpu_some=$(awk '$1=="some"{print $2}' /proc/pressure/cpu | sed 's/avg10=//;s/%//')
  io_some=$(awk '$1=="some"{print $2}' /proc/pressure/io | sed 's/avg10=//;s/%//')
  io_full=$(awk '$1=="full"{print $2}' /proc/pressure/io | sed 's/avg10=//;s/%//')
  if [ "$compilers" -eq 0 ] && \
     awk -v c="$cpu_some" 'BEGIN{exit !(c<20)}' && \
     awk -v s="$io_some" 'BEGIN{exit !(s<20)}' && \
     awk -v f="$io_full" 'BEGIN{exit !(f<10)}'; then
    break
  fi
  sleep 20
done

echo "admission open; running the engine test"
CARGO_TARGET_DIR=/var/tmp/fc-rewind-target cargo test -p engine --test history_rewind 2>&1
echo "exit=$?"
