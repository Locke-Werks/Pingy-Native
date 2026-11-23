# See packaging/linux/debian/postinst for why the capability is needed.
if command -v setcap >/dev/null 2>&1; then
    setcap cap_net_raw+ep /usr/bin/pingy || \
        echo "pingy: could not set cap_net_raw; pings may fail unless ping_group_range includes your group" >&2
fi
