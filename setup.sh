#!/bin/bash
set -e

echo "=== VibeCast Arch Linux Setup Script ==="

# 1. Install required packages
echo "Installing prerequisites (NetworkManager, Bluez, PipeWire)..."
sudo pacman -S --needed --noconfirm networkmanager bluez bluez-utils pipewire pipewire-pulse wireplumber

# 2. Add Udev Rule for persistent Wi-Fi naming (wlan0)
# We match by devtype "wlan" so the first Wi-Fi interface always gets named wlan0.
echo "Creating udev rule for persistent wlan0 naming..."
echo 'SUBSYSTEM=="net", ACTION=="add", DRIVERS=="?*", ATTR{type}=="1", NAME="wlan0"' | sudo tee /etc/udev/rules.d/70-persistent-net.rules > /dev/null
sudo udevadm control --reload-rules
sudo udevadm trigger

# 3. Create the 'vibecast' user if it doesn't exist
if ! id "vibecast" &>/dev/null; then
    echo "Creating 'vibecast' user..."
    sudo useradd -m -s /bin/bash vibecast
fi

# 4. Configure D-Bus permissions for the 'vibecast' user
echo "Configuring D-Bus system policies for system-bridge..."
sudo cp packaging/arch/com.vibecast.SystemBridge.conf /etc/dbus-1/system.d/
# Reload D-Bus to ingest new policy
sudo systemctl reload dbus

# 5. Enable System Services
echo "Enabling and starting NetworkManager, Bluetooth, and VibeCast services..."
sudo systemctl enable --now NetworkManager
sudo systemctl enable --now bluetooth

# Enable remote-server and system-bridge
# (Assuming they are properly copied to /usr/bin and /etc/systemd/system by the PKGBUILD)
echo "Enabling VibeCast System Bridge and Remote Server..."
sudo systemctl enable system-bridge.service
sudo systemctl enable vibecast-remote-server.service || echo "Warning: vibecast-remote-server.service not found yet"

echo "=== Setup Complete! ==="
echo "Please reboot your system for udev rules and user groups to take full effect."
