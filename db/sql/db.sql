create user files with createdb password 'files';
create database files_db owner files encoding utf8;

create table authentication_type (
	id smallint primary key,
	name varchar not null
);

insert into authentication_type values ( 1, 'password' );

create table authentication_password (
	id bigserial,
	hash bytea not null,
	constraint pk_auth_passworkd primary key (id)
);

create table users (
   id bigserial,
   name varchar not null,
   constraint pk_users primary key (id)
);

CREATE UNIQUE INDEX user_name_idx ON users (name);

create table authentication (
	user_id bigint not null,
	auth_type_id smallint not null,
	auth_password_id bigint null,
	constraint fk_users foreign key (user_id) references users(id) on delete cascade,
	constraint fk_auth_type foreign key (auth_type_id) references authentication_type(id),
	constraint fk_auth_pass foreign key (auth_password_id) references authentication_password(id) on delete set null
);

create table roles (
	id serial,
	name varchar(200) not null,
	constraint pk_roles primary key (id)
);

create table user_roles (
	role_id int not null,
	user_id int not null,
	constraint pk_user_roles primary key (role_id, user_id),
	constraint fk_roles foreign key (role_id) references roles(id),
	constraint fk_users foreign key (user_id) references users(id) on delete cascade
);


create table directories (
	id bigint,
	user_id bigint not null,
	title varchar not null,
	parent_id bigint,
	level int not null,
	filesystem_path varchar not null,
	constraint pk_directories primary key (id, user_id),
	constraint fk_users foreign key (user_id) references users(id),
	constraint fk_directories foreign key (parent_id, user_id) references directories(id, user_id)	
) partition by hash (user_id);

--comment on directories.level is 'directory hierarchy level. Top level directories has value 0';
--comment on directories.filesystem_path is 'physical directory location on filesystem';

create table file_nodes (
	id bigint not null,
	user_id bigint not null,
	title varchar not null,
	parent_id bigint not null,
	filesystem_path varchar not null,
	version int not null,
	constraint pk_file_nodes primary key (id, user_id),
	constraint fk_directories foreign key (parent_id, user_id) references directories(id, user_id)
) partition by hash (user_id);

--create table file_versions (
--	file_id uuid,
--	version int,
--	
--	filesystem_path varchar not null,
--)
-- These sequences needs to be create per user ( when user is created )
--create sequence user_directory_1 as bigint increment by 1 minvalue 0 NO MAXVALUE no cycle;
--create sequence user_files_1 as bigint increment by 1 minvalue 0 NO MAXVALUE no cycle;







