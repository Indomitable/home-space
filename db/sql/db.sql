create user files with createdb password 'files';
create database files_db owner files encoding utf8;

create table authentication_type (
	id smallint primary key,
	name varchar not null
);

insert into authentication_type values ( 1, 'password' );

create table users (
   id bigserial,
   name varchar not null,
   constraint pk_users primary key (id)
);

CREATE UNIQUE INDEX user_name_idx ON users (name);

create table authentication_password (
	id bigserial,
	hash varchar not null,
	constraint pk_auth_passworkd primary key (id)
);

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

--comment on directories.level is 'directory hierarchy level. Top level directories has value 0';
--comment on directories.filesystem_path is 'physical directory location on filesystem';

drop table file_nodes;

create table file_nodes (
	id bigint not null,
	user_id bigint not null,
	title varchar not null,
	parent_id bigint null,
	node_type SMALLINT NOT NULL,	
	filesystem_path varchar not null,
	mime_type varchar,
	constraint pk_file_nodes primary key (id, user_id),
	constraint fk_file_nodes foreign key (parent_id, user_id) references file_nodes(id, user_id)
);

create index idx_file_nodes on file_nodes (user_id);

create sequence file_nodes_user_1 as bigint increment by 1 minvalue 1 NO MAXVALUE no cycle owned by file_nodes.id;

insert into file_nodes (id, user_id, title, parent_id, node_type, filesystem_path)
values (0, 1, 'ROOT', null, 0, '/mnt/storage/files/1');

select ap.hash from authentication_password ap
	inner join authentication a on a.auth_password_id  = ap.id 
	inner join users u on u.id  = a.user_id 
	where u."name" = $1

	
drop table authentication;
drop table authentication_password;

--create table file_versions (
--	file_id uuid,
--	version int,
--	
--	filesystem_path varchar not null,
--)
-- These sequences needs to be create per user ( when user is created )
--create sequence user_directory_1 as bigint increment by 1 minvalue 0 NO MAXVALUE no cycle;
--create sequence user_files_1 as bigint increment by 1 minvalue 0 NO MAXVALUE no cycle;







