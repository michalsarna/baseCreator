#!/bin/zsh
# The performance test of LightCreator, MillCreator and PenCreator on the shared test project, before every release.
#   scripts/perf.sh            measure and add the results to testdata/perf/RESULTS.md
#   scripts/perf.sh --check    only check that ~/Documents/test/perf-block.cre is the same as the file of the repository
# The repositories of the programs lie beside this one. The copy in ~/Documents/test is made when it is missing; when it differs from
# the file of the repository the script stops: the file of the repository is the test project, change it only with
# crates/base-core/examples/perf_project.rs and the perf_prepare_the_project tests of the programs, then copy it there again.
set -e
BASE=${0:A:h:h}
ROOT=${BASE:h}
FILE=$BASE/testdata/perf/perf-block.cre
COPY=$HOME/Documents/test/perf-block.cre
if [[ ! -f $COPY ]]; then
  mkdir -p ${COPY:h}
  cp $FILE $COPY
  echo "made the copy $COPY"
elif ! cmp -s $FILE $COPY; then
  echo "$COPY differs from $FILE:"
  shasum -a 256 $FILE $COPY
  echo "make them the same first (cp the file of the repository there)."
  exit 1
fi
echo "the test project and its copy are the same ($(shasum -a 256 $FILE | cut -c1-12))"
[[ $1 == --check ]] && exit 0
SHA=$(shasum -a 256 $FILE | cut -c1-12)
CPU=$(sysctl -n machdep.cpu.brand_string 2>/dev/null || uname -m)
OUT=$(mktemp)
for app in LightCreator MillCreator PenCreator; do
  echo "== $app"
  (cd $ROOT/$app && CREATOR_PERF_FILE=$FILE cargo test -q --release -p ${app:l} perf_open -- --ignored --nocapture --test-threads=1 | grep '^PERF' | tee -a $OUT)
done
DAY=$(date +%F)
while IFS=$'\t' read -r _ app ver what ms detail; do
  echo "| $DAY | $app $ver | $what | $ms | $detail | $CPU | $SHA |"
done < $OUT >> $BASE/testdata/perf/RESULTS.md
rm -f $OUT
echo "added to $BASE/testdata/perf/RESULTS.md"
