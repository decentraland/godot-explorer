FROM ubuntu:24.04

# The image tag is a hash of this directory (see android_builds.yml). Bump to rebuild with fresh
# apt / sdkmanager packages without changing anything else.
ARG IMAGE_REVISION=1

ENV ANDROID_SDK_MAJOR_VERSION=35
ENV ANDROID_NDK_VERSION=28.1.13356709
ENV ANDROID_SDK_CMAKE_VERSION=3.30.5

ENV ANDROID_HOME=/opt/android-sdk-linux
ENV ANDROID_SDK_HOME=${ANDROID_HOME}
ENV ANDROID_SDK_ROOT=${ANDROID_HOME}
ENV ANDROID_SDK=${ANDROID_HOME}
ENV ANDROID_NDK_HOME=${ANDROID_HOME}/ndk/${ANDROID_NDK_VERSION}
ENV ANDROID_NDK_ROOT=${ANDROID_NDK_HOME}
ENV ANDROID_NDK=${ANDROID_NDK_HOME}

ENV JAVA_HOME=/usr/lib/jvm/java-17-openjdk-amd64
ENV PATH="$JAVA_HOME/bin:${ANDROID_HOME}/cmdline-tools/latest/bin:$PATH"

RUN dpkg --add-architecture i386 \
    && apt-get update -yqq \
    && apt-get install -y \
        openjdk-17-jdk libc6:i386 libgcc1:i386 libncurses6:i386 libstdc++6:i386 zlib1g:i386 \
        xvfb libasound2-dev libudev-dev clang curl pkg-config \
        libssl-dev libx11-dev libgl1-mesa-dev libxext-dev gnupg wget unzip git build-essential \
        autoconf texinfo gcc-multilib \
    && rm -rf /var/lib/apt/lists/* /var/cache/apt/*

# Command-line tools, then one RUN per heavy package so each is its own layer (pulled in parallel).
RUN mkdir -p ${ANDROID_HOME}/cmdline-tools \
    && wget -q https://dl.google.com/android/repository/commandlinetools-linux-11076708_latest.zip -O /tmp/cmdline-tools.zip \
    && unzip -q /tmp/cmdline-tools.zip -d /tmp/cmdline-tools \
    && mv /tmp/cmdline-tools/cmdline-tools ${ANDROID_HOME}/cmdline-tools/latest \
    && rm -rf /tmp/cmdline-tools /tmp/cmdline-tools.zip \
    && yes | sdkmanager --licenses > /dev/null

# build-tools 33.0.1 / 35.0.1 and platform 33 are what Godot's export and Gradle otherwise
# download on every run.
RUN sdkmanager "platform-tools" \
        "platforms;android-${ANDROID_SDK_MAJOR_VERSION}" "platforms;android-33" \
        "build-tools;34.0.0" "build-tools;33.0.1" "build-tools;35.0.1" \
        "cmake;${ANDROID_SDK_CMAKE_VERSION}"
RUN sdkmanager "ndk;${ANDROID_NDK_VERSION}" --channel=3

WORKDIR /app

ENTRYPOINT ["/bin/bash"]
