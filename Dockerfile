FROM ubuntu:latest

RUN apt-get update && apt-get upgrade -y

RUN apt-get install -y --no-install-recommends \
    xvfb libasound2-dev libudev-dev \
    clang curl pkg-config libavcodec-dev libavformat-dev libavutil-dev libavfilter-dev libavdevice-dev \
    libssl-dev libx11-dev libgl1-mesa-dev libxext-dev

WORKDIR /app

COPY exports/decentraland.godot.client.x86_64 \
    exports/decentraland.godot.client.pck \
    exports/libdecentraland_godot_lib.so  \
    entry-point.sh \
    /app/

RUN chmod +x entry-point.sh

ENTRYPOINT ["./entry-point.sh"]
