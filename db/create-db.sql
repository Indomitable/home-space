drop database if exists files_db;
drop user if exists files;

create user files with createdb password 'files';
create database files_db owner files encoding utf8;
