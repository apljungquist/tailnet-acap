#!/usr/bin/env sh
set -eux

VIRTUAL_ENV="${1}"
INIT_ENV="${2:-}"

# Ensure the existence of venv.
# This is used below for local installations of applications and libraries.
python3 -m venv "${VIRTUAL_ENV}"

. "${VIRTUAL_ENV}/bin/activate"

# Install python programs and packages
# As of writing this is needed because:
# - the system version of `jsonschema` on Bookworm is 4.10.3, but the SDK uses 3.2.0, which
#   presumably is incompatible.
PIP_CONSTRAINT=constraints.txt pip install --requirement requirements.txt

# Install rust programs
cargo install --locked --root "${VIRTUAL_ENV}" --target-dir /tmp/target cargo-about@0.6.2
cargo install --locked --root "${VIRTUAL_ENV}" --target-dir /tmp/target mkhelp@0.2.3
cargo install --locked --root "${VIRTUAL_ENV}" --target-dir /tmp/target --git https://github.com/AxisCommunications/acap-rs.git --rev 37294a76dc24369b1d971504c41da37fe34394f2 cargo-acap-build
cargo install --locked --root "${VIRTUAL_ENV}" --target-dir /tmp/target --git https://github.com/AxisCommunications/acap-rs.git --rev 37294a76dc24369b1d971504c41da37fe34394f2 cargo-acap-sdk

rm -r /tmp/target

# Create `init_env.sh` in a location where it can be sourced conveniently.
if [ ! -z "${INIT_ENV}" ];
then
  {
    echo "# Automatically created by install-venv.sh";
    echo ". ${VIRTUAL_ENV}/bin/activate";
    echo "unset -f deactivate";
    echo 'cargo-acap-sdk completions $(basename $SHELL) | . /dev/stdin'
    echo alias asdk=cargo-acap-sdk
    cat environment-setup.sh;
  } > "${INIT_ENV}"
fi
