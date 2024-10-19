FROM ubuntu:22.04

RUN apt-get update && apt-get upgrade -y

RUN apt-get install -y --no-install-recommends \
    xvfb libasound2-dev libudev-dev \
    clang curl pkg-config libavcodec-dev libavformat-dev libavutil-dev libavfilter-dev libavdevice-dev \
    libssl-dev libx11-dev libgl1-mesa-dev libxext-dev

WORKDIR /app

COPY decentraland.godot.client.x86_64 \
    decentraland.godot.client.pck \
    libdclgodot.so  \
    entry-point.sh \
    /app/

RUN chmod +x entry-point.sh
RUN chmod +x decentraland.godot.client.x86_64

ENTRYPOINT ["./entry-point.sh"]
