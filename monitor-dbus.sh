#!/bin/bash

            

DBUS_INTERFACE="org.freedesktop.Notifications"



dbus-monitor --session "interface=org.freedesktop.Notifications" | while
read -r line; do


echo "${line}"


done
          


