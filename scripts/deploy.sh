#!/bin/bash

mkdir -p publish
rm -rf publish/*
# build
dotnet build -c Release -r linux-arm64 backend/HomeSpace.Api/HomeSpace.Api.csproj
# publish
dotnet publish -c Release -r linux-arm64 backend/HomeSpace.Api/HomeSpace.Api.csproj -o publish/

#go to client folder
cd client
# build client
npm run build
# go back to the root
cd ..
# copy client dist to publish
cp -r client/dist publish/wwwroot

# copy published to rpi
# scp publish/* ubuntu@192.168.1.144:./home-space-cs/
rsync -av --progress --delete --exclude=appsettings.Production.json  publish/  ubuntu@192.168.1.144:./home-space-cs/

