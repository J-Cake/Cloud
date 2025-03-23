use clap::{
	arg,
	Parser,
	Subcommand
};
use serde::{
	Deserialize,
	Serialize
};
use std::{
	path::Path,
	path::Component,
	os::unix::fs::MetadataExt,
	iter,
	io::Write,
	io::Result,
	io::Read,
	io::ErrorKind,
	io::Error,
	io,
	fs::OpenOptions,
	fs::Metadata,
	fs,
	path::PathBuf,
	time::SystemTime
};
use std::process::exit;

#[derive(Parser, Debug, Clone)]
pub struct Args {
	#[arg(required = true)]
	uid: u32,

	#[arg(long, default_value = "/")]
	base: PathBuf,

	#[command(subcommand)]
	action: Action,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Action {
	#[clap(name = "file::read")]
	FileRead {
		path: PathBuf
	},

	#[clap(name = "file::write")]
	FileWrite {
		path: PathBuf,
		create: Option<bool>
	},

	#[clap(name = "file::mkdir")]
	Mkdir {
		path: PathBuf
	},

	#[clap(name = "file::lsdir")]
	Lsdir {
		path: PathBuf,

		#[clap(long = "depth")]
		max_depth: Option<u32>
	},

	#[clap(name = "file::rm")]
	Remove {
		path: PathBuf
	},

	#[clap(name = "file::rename")]
	Move {
		path: PathBuf,
		to: PathBuf
	},

	#[clap(name = "file::copy")]
	Copy {
		path: PathBuf,
		to: PathBuf
	},

	#[clap(name = "file::metadata")]
	Meta {
		path: PathBuf
	},

	#[clap(name = "file::write_metadata")]
	WriteMeta {
		path: PathBuf,
	}
}

impl Action {
	pub fn set_base(mut self, base: impl AsRef<Path>) -> Self {
		match self {
			Action::FileRead { ref mut path, .. } |
			Action::FileWrite { ref mut path, .. } |
			Action::Mkdir { ref mut path, .. } |
			Action::Lsdir { ref mut path, .. } |
			Action::Remove { ref mut path, .. } |
			Action::Move { ref mut path, .. } |
			Action::Copy { ref mut path, .. } |
			Action::Meta { ref mut path, .. } |
			Action::WriteMeta { ref mut path, .. } => {
				*path = base.as_ref().join(path.flatten()
					.components()
					.map(|i| if i == Component::RootDir { Component::CurDir } else { i })
					.collect::<PathBuf>())
					.flatten()
			},
		}

		self
	}

	pub fn print(self) -> Self {
		eprintln!("Invoked Agent: {:?}", &self);
		self
	}
}

pub trait Flatten where Self: AsRef<Path> {
	fn flatten(self) -> PathBuf;
}

impl<P: AsRef<Path>> Flatten for P {
	fn flatten(self) -> PathBuf {
		let mut chunks = Vec::new();

		for chunk in self.as_ref()
			.components() {

			if chunk == Component::ParentDir {
				chunks.pop();
			} else if chunk != Component::CurDir {
				chunks.push(chunk);
			}
		}

		chunks.into_iter()
			.collect()
	}
}

pub fn seteuid(uid: u32) -> Result<()> {
	match unsafe { libc::seteuid(uid) } {
		0 => Ok(()),
		e => Err(Error::from_raw_os_error(e))
	}
}

pub fn pipe(mut from: impl Read, mut to: impl Write) -> Result<()> {
	let mut buf = vec![0u8; 1024 ^ 2];

	loop {
		let len = from.read(&mut buf)?;

		if len == 0 {
			break;
		}

		to.write_all(&buf[..len])?;
	}

	Ok(())
}

fn main() -> Result<()> {
	let args = Args::parse();

	seteuid(args.uid)?;

	match args.action.set_base(&args.base).print() {
		Action::FileRead { path } => pipe(
		OpenOptions::new()
				.read(true)
				.open(path)?,
			io::stdout()
		)?,

		Action::FileWrite { path, create } => pipe(
			io::stdin(),
			OpenOptions::new()
				.write(true)
				.create(create.unwrap_or(false))
				.open(path)?
		)?,

		Action::Mkdir { path } => fs::create_dir_all(path)?,

		Action::Lsdir { path, max_depth } => {
			fn walk(path: impl AsRef<Path>, max_depth: u32) -> Result<Box<dyn Iterator<Item = DirEntry>>> {
				if max_depth <= 0 {
					return Ok(Box::new(iter::empty()));
				}

				let _ = path.as_ref().metadata()?;

				Ok(Box::new(fs::read_dir(path)?
					.filter_map(move |dir| dir.ok())
					.filter_map(move |entry| Some(match entry.metadata().ok()? {
						meta if meta.is_dir() => Box::new(iter::once(DirEntry::dir(entry.path()).ok()?)
							.chain(walk(entry.path(), max_depth - 1).ok()?))
							as Box<dyn Iterator<Item = DirEntry>>,
						meta if meta.is_file() => Box::new(iter::once(DirEntry::file(entry.path(), meta).ok()?))
							as Box<dyn Iterator<Item = DirEntry>>,
						_ => return None
					}))
					.flatten()))
			}

			if path.exists() {
				for dir in walk(path, max_depth.unwrap_or(u32::MAX))? {
					println!("{}", serde_json::to_string(&dir.relative_to(&args.base)?)?);
				}
			} else {
				exit(libc::ENOENT);
			}
		},

		Action::Remove { path } => rm(path)?,
		Action::Copy { path, to } => copy(path, to)?,

		Action::Move { path, to } => match fs::rename(&path, &to) {
			Ok(()) => (),
			Err(err) if err.kind() == ErrorKind::CrossesDevices => {
				copy(&path, to)?;
				rm(path)?;
			},
			Err(e) => Err(e)?
		},

		Action::Meta { .. } => todo!(),
		Action::WriteMeta { .. } => todo!(),
	};

	Ok(())
}

pub fn copy(from: impl AsRef<Path>, to: impl AsRef<Path>) -> Result<()> {
	if from.as_ref().is_dir() {
		for child in from.as_ref().read_dir()? {
			let child = child?;
			let to = to.as_ref().join(child.file_name());
			copy(child.path(), to)?;
		}

		Ok(())
	} else if from.as_ref().is_symlink() {
		std::os::unix::fs::symlink(from.as_ref().read_link()?, &to)?;
		Ok(())
	} else if from.as_ref().is_file() {
		pipe(OpenOptions::new()
			.read(true)
			.open(from.as_ref())?,  OpenOptions::new()
			.write(true)
			.create(true)
			.open(to.as_ref())?)?;
		Ok(())
	} else {
		Err(Error::from(ErrorKind::InvalidInput))
	}
}

pub fn rm(from: impl AsRef<Path>) -> Result<()> {
	match from.as_ref() {
		path if path.is_dir() => fs::remove_dir_all(path),
		path => fs::remove_file(path)
	}
}

#[derive(Serialize, Deserialize)]
enum DirEntry {
	Dir(PathBuf),
	File {
		path: PathBuf,
		size: usize,
		modified: SystemTime,
		created: SystemTime,
	},
}

impl DirEntry {
	pub fn file(dir: impl AsRef<Path>, metadata: Metadata) -> Result<Self> {
		Ok(Self::File {
			path: dir.as_ref().to_path_buf(),

			size: metadata.size() as usize,
			modified: metadata.modified()?,
			created: metadata.created()?,
		})
	}

	pub fn dir(dir: impl AsRef<Path>) -> Result<Self> {
		Ok(Self::Dir(dir.as_ref().to_path_buf()))
	}

	pub fn relative_to(mut self, base: impl AsRef<Path>) -> Result<Self> {
		match self {
			Self::File { ref mut path, .. } | Self::Dir(ref mut path) => *path = PathBuf::from("/").join(path.strip_prefix(&base)
				.map_err(|err| Error::new(ErrorKind::InvalidInput, err))
				?.to_path_buf())
		}

		Ok(self)
	}
}