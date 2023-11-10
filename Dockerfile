FROM ubuntu:latest

RUN apt-get update && apt-get upgrade -y

RUN apt-get install -y --no-install-recommends \
    xvfb libasound2-dev libudev-dev \
    clang curl pkg-config libavcodec-dev libavformat-dev libavutil-dev libavfilter-dev libavdevice-dev \
    libssl-dev libx11-dev libgl1-mesa-dev libxext-dev

WORKDIR /app

COPY exports/decentraland.godot.client.x86_64 exports/decentraland.godot.client.pck exports/libdecentraland_godot_lib.so  /app/

RUN <<EOF
echo "#!/bin/sh" > entry-point.sh
echo "" >> entry-point.sh
echo "/usr/bin/Xvfb -ac :99 -screen 0 1280x1024x24 &" >> entry-point.sh
echo "export DISPLAY=:99" >> entry-point.sh
echo "./decentraland.godot.client.x86_64 --rendering-driver opengl3 --avatar-renderer --avatars avatars.json" >> entry-point.sh
EOF

RUN chmod +x entry-point.sh

ENTRYPOINT ["./entry-point.sh"]
