FROM ubuntu:24.04

RUN apt-get update && apt-get upgrade -y

RUN apt-get install -y --no-install-recommends \
    xvfb libasound2-dev libudev-dev \
    clang curl pkg-config \
    libssl-dev libx11-dev libgl1-mesa-dev libxext-dev \
    libxcursor1 libxinerama1 libxrandr2 libxi6 libwayland-cursor0 \
    libdbus-1-3 libxrender1 libxkbcommon0 libfontconfig1 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY decentraland.godot.client.x86_64 \
    decentraland.godot.client.pck \
    libdclgodot.so  \
    libsentry.linux.debug.x86_64.so  \
    crashpad_handler  \
    entry-point.sh \
    /app/

RUN chmod +x entry-point.sh
RUN chmod +x decentraland.godot.client.x86_64

ENTRYPOINT ["./entry-point.sh"]
