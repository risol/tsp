#!/bin/bash
# Fix Samba AD network binding configuration
# Allow external connections to LDAP service

# Wait for Samba configuration to fully generate
echo "Waiting for Samba configuration to generate..."
sleep 15

# Check if config file exists
if [ -f /usr/local/samba/etc/smb.conf ]; then
    echo "Fixing Samba network binding configuration..."

    # Modify config to allow listening on all network interfaces
    sed -i 's/bind interfaces only = Yes/bind interfaces only = No/' /usr/local/samba/etc/smb.conf

    # Stop all Samba processes
    echo "Restarting Samba service..."
    killall samba || true
    sleep 2

    # Restart Samba (will start automatically in background)
    /usr/local/samba/sbin/samba -D

    echo "Samba configuration updated, now listening on all network interfaces"
else
    echo "Config file does not exist, waiting for Samba to initialize..."
fi
