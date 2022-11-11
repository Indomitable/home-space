create user files with createdb password 'files';
create database files_db owner files encoding utf8;

create table users (
   id bigserial,
   name varchar not null,
   constraint pk_users primary key (id)
);

CREATE UNIQUE INDEX user_name_idx ON users (name);

create table authentication_type (
	id smallint primary key,
	name varchar not null
);

insert into authentication_type values ( 1, 'password' );


create table authentication_password (
	id bigserial,
	hash bytea not null,
	salt bytea not null,
	constraint pk_auth_passworkd primary key (id)
);

create table authentication (
	user_id bigint not null,
	auth_type_id smallint not null,
	auth_id bigint null,
	constraint fk_users foreign key (user_id) references users(id) on delete cascade,
	constraint fk_auth_type foreign key (auth_type_id) references authentication_type(id)
);

create table refresh_tokens
(
    token varchar not null,
    user_id bigint not null,
    valid_to timestamptz not null,
    constraint pk_refresh_tokens primary key (token),
    constraint fk_users foreign key (user_id) references users(id) on delete cascade
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

create table file_nodes (
	id bigint not null,
	user_id bigint not null,
	title varchar not null,
	parent_id bigint null,
	node_type SMALLINT NOT NULL,	
	filesystem_path varchar not null,
	mime_type varchar not null,
	modified_at timestamptz not null,
	node_size bigint not null,
	node_version int not null,
	hashsum bytea NULL,
	constraint pk_file_nodes primary key (id, user_id),
	constraint fk_file_nodes foreign key (parent_id, user_id) references file_nodes(id, user_id)
);

create index idx_file_nodes on file_nodes (user_id);
create index idx_file_nodes_paths on file_nodes (user_id, filesystem_path);

create table file_versions (
	id bigint not null,
	user_id bigint not null,
	node_version int not null,
	created_at timestamptz not null,
	node_size bigint not null,
	file_name varchar not null,
	hashsum bytea NULL;
	constraint pk_file_versions primary key (id, user_id, node_version),
	constraint fk_file_versions foreign key (id, user_id) references file_nodes(id, user_id)
);

create index idx_file_versions on file_versions (user_id);

create table trash_box (
	id bigint not null,
	user_id bigint not null,
	title varchar not null,
	parent_id bigint NOT NULL,
	node_type SMALLINT NOT NULL,	
	filesystem_path varchar not null,
	mime_type varchar not null,
	version_created_at timestamptz not null, 
	deleted_at timestamptz not null,
	node_size bigint not null,
	node_version int not null,
	file_name varchar not null,
	constraint pk_trash_box primary key (id, user_id, node_version)	
);

create index idx_trash_box on trash_box (user_id);

CREATE TABLE public.favorite_nodes (
	id int8 NOT NULL,
	user_id int8 NOT NULL,
	CONSTRAINT pk_favorite_nodes PRIMARY KEY (id, user_id),
	CONSTRAINT fk_favorite_nodes_file_nodes FOREIGN KEY (id,user_id) REFERENCES public.file_nodes(id,user_id)
);

