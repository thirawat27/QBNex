#!/bin/bash
#QBNex Installer -- Shell Script -- Matt Kilgore 2013
#Version 5 -- January 2020

#This checks the currently installed packages for the ones QBNex needs
#And runs the package manager to install them if that is the case
pkg_install() {
  #Search
  packages_to_install=
  for pkg in $pkg_list; do
    if [ -z "$(echo "$installed_packages" | grep $pkg)" ]; then
      packages_to_install="$packages_to_install $pkg"
    fi
  done
  if [ -n "$packages_to_install" ]; then
    echo "Installing required packages. If prompted to, please enter your password."
    $installer_command $packages_to_install
  fi

}



#Make sure we're not running as root
if [ $EUID == "0" ]; then
  echo "You are trying to run this script as root. This is highly unrecommended."
  echo "This script will prompt you for your sudo password if needed to install packages."
  exit 1
fi

GET_WGET=
#Path to Icon
#Relative Path to icon -- Don't include beginning or trailing '/'
QBNex_ICON_PATH="internal/source"

#Name of the Icon picture
QBNex_ICON_NAME="qbnexicon32.png"

DISTRO=

lsb_command=`which lsb_release 2> /dev/null`
if [ -z "$lsb_command" ]; then
  lsb_command=`which lsb_release 2> /dev/null`
fi

#Outputs from lsb_command:

#Arch Linux  = arch
#Debian      = debian
#Fedora      = Fedora
#KUbuntu     = ubuntu
#LUbuntu     = ubuntu
#Linux Mint  = linuxmint
#Ubuntu      = ubuntu
#Slackware   = slackware
#VoidLinux   = voidlinux
#XUbuntu     = ubuntu
#Zorin       = Zorin
if [ -n "$lsb_command" ]; then
  DISTRO=`$lsb_command -si | tr '[:upper:]' '[:lower:]'`
elif [ -e /etc/arch-release ]; then
  DISTRO=arch
elif [ -e /etc/debian_version ] || [ -e /etc/debian_release ]; then
  DISTRO=debian
elif [ -e /etc/fedora-release ]; then
  DISTRO=fedora
elif [ -e /etc/redhat-release ]; then
  DISTRO=redhat
elif [ -e /etc/centos-release ]; then
  DISTRO=centos
fi

#Find and install packages
if [ "$DISTRO" == "arch" ]; then
  echo "ArchLinux detected."
  pkg_list="gcc zlib xorg-xmessage $GET_WGET"
  installed_packages=`pacman -Q`
  installer_command="sudo pacman -S "
  pkg_install
elif [ "$DISTRO" == "linuxmint" ] || [ "$DISTRO" == "ubuntu" ] || [ "$DISTRO" == "debian" ] || [ "$DISTRO" == "zorin" ]; then
  echo "Debian based distro detected."
  pkg_list="g++ x11-utils mesa-common-dev libglu1-mesa-dev libasound2-dev zlib1g-dev $GET_WGET"
  installed_packages=`dpkg -l`
  installer_command="sudo apt-get -y install "
  pkg_install
elif [ "$DISTRO" == "fedora" ] || [ "$DISTRO" == "redhat" ] || [ "$DISTRO" == "centos" ]; then
  echo "Fedora/Redhat based distro detected."
  pkg_list="gcc-c++ xmessage mesa-libGLU-devel alsa-lib-devel zlib-devel $GET_WGET"
  installed_packages=`yum list installed`
  installer_command="sudo yum install "
  pkg_install
elif [ "$DISTRO" == "voidlinux" ]; then
   echo "VoidLinux detected."
   pkg_list="gcc xmessage glu-devel zlib-devel alsa-lib-devel $GET_WGET"
   installed_packages=`xbps-query -l |grep -v libgcc`
   installer_command="sudo xbps-install -Sy "
   pkg_install

elif [ -z "$DISTRO" ]; then
  echo "Unable to detect distro, skipping package installation"
  echo "Please be aware that QBNex requires the following to compile:"
  echo "  OpenGL developement libraries"
  echo "  ALSA development libraries"
  echo "  GNU C++ Compiler (g++)"
  echo "  xmessage (x11-utils)"
  echo "  zlib"
fi

echo "Compiling and installing QBNex..."

### Build process
find . -name "*.sh" -exec chmod +x {} \;
find internal/c/parts -type f -iname "*.a" -exec rm -f {} \;
find internal/c/parts -type f -iname "*.o" -exec rm -f {} \;
find internal/c/libqb -type f -iname "*.o" -exec rm -f {} \;
mkdir -p ./internal/temp
rm -f ./internal/temp/*

echo "Building library 'LibQB'"
pushd internal/c/libqb/os/lnx >/dev/null
rm -f libqb_setup.o
./setup_build.sh
popd >/dev/null

echo "Building library 'FreeType'"
pushd internal/c/parts/video/font/ttf/os/lnx >/dev/null
rm -f src.o
./setup_build.sh
popd >/dev/null

echo "Building library 'Core:FreeGLUT'"
pushd internal/c/parts/core/os/lnx >/dev/null
rm -f src.a
./setup_build.sh
popd >/dev/null

echo "Building 'QBNex'"
cp -r ./internal/source/* ./internal/temp/
pushd internal/c >/dev/null
g++ -no-pie -w qbx.cpp libqb/os/lnx/libqb_setup.o parts/video/font/ttf/os/lnx/src.o parts/core/os/lnx/src.a -lGL -lGLU -lX11 -lpthread -ldl -lrt -D FREEGLUT_STATIC -o ../../qb
popd

if [ -e "./qb" ]; then
  echo "Done compiling!!"
  echo
  echo "QBNex CLI compiler is ready:"
  echo "  ./qb yourfile.bas"
else
  ### QBNex did not compile
  echo "It appears that the qb executable file was not created, which usually indicates a compile failure."
  echo "Usually these are due to missing packages needed for compilation. If you're not running a distro supported by this compiler, please note you will need to install the packages listed above."
  echo "If you need help, please open an issue at https://github.com/thirawat27/QBNex/issues with your distro details and build log."
  echo "Also, please tell them the exact contents of this next line:"
  echo "DISTRO: $DISTRO"
fi
echo
echo "Thank you for using the QBNex installer."
