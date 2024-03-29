#!/bin/bash

DEPLOY_SERVER=$1
if [ -z $DEPLOY_SERVER ];
then
  echo No server provided
  exit
fi

mkdir -p publish
rm -rf publish/*
# build
dotnet build -c Release -r linux-arm64 --self-contained backend/HomeSpace.Api/HomeSpace.Api.csproj || exit
# publish
dotnet publish -c Release -r linux-arm64 --no-build backend/HomeSpace.Api/HomeSpace.Api.csproj -o publish/

#go to client folder
cd client || exit 
# build client
npm run build || exit
# go back to the root
cd .. || exit
# copy client dist to publish
cp -r client/dist publish/wwwroot
cp backend/home-space.service publish/

# copy published to rpi
# scp publish/* ubuntu@192.168.1.144:./home-space-cs/
rsync -av --progress --delete --exclude=appsettings.Production.json  publish/  ubuntu@"$DEPLOY_SERVER":./home-space/

