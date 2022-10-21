#!/bin/bash

# build
dotnet build -c Release -r linux-arm64 HomeSpace.Api/HomeSpace.Api.csproj
# publish
dotnet publish -c Release -r linux-arm64 HomeSpace.Api/HomeSpace.Api.csproj -o publish/
# copy published to rpi
# scp publish/* ubuntu@192.168.1.144:./home-space-cs/
rsync -av --progress --delete --exclude=appsettings.Production.json  publish/  ubuntu@192.168.1.144:./home-space-cs/

