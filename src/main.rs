extern crate iron;
extern crate mount;
extern crate reqwest;
extern crate serde;
extern crate serde_json;
extern crate staticfile;

use std::fs;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;
use std::path::PathBuf;

use iron::prelude::*;
use iron::AfterMiddleware;
use mount::Mount;
use serde::Deserialize;
use staticfile::Static;
use structopt::StructOpt;

#[derive(StructOpt)]
struct Opt {
    /// CircleCI API Token
    #[structopt(long, env = "CIRCLE_TOKEN")]
    circle_token: String,

    /// Return /static/path/to/assets file when /**/static/path/to/assets is requested.
    /// This may be helpful when loading assets files from relative path.
    #[structopt(long)]
    handle_assets: bool,

    /// Skip download artifacts, and launch server
    #[structopt(long)]
    skip_download: bool,

    /// Default current directory
    #[structopt(long, parse(from_os_str))]
    directory: Option<PathBuf>,

    #[structopt(long, default_value = "3000")]
    port: u32,

    /// VCS type supported by CircleCI. Currently 'github' or 'bitbucket'
    #[structopt(name = "vcs")]
    vcs: String,

    /// User or Organization
    #[structopt(name = "USER")]
    user: String,

    /// CircleCI project name
    #[structopt(name = "PROJECT")]
    project: String,

    /// Build number of CircleCI job
    #[structopt(name = "BUILD_NUM")]
    build_num: u32,
}

fn main() {
    let opt = Opt::from_args();

    let downloaded_dir = if opt.skip_download {
        downloaded_dir(opt.vcs, opt.user, opt.project, opt.build_num, opt.directory)
    } else {
        download_artifacts(
            opt.vcs,
            opt.user,
            opt.project,
            opt.build_num,
            opt.circle_token,
            opt.directory,
        )
        .unwrap()
    };

    let mut mount = Mount::new();
    mount.mount("/", Static::new(downloaded_dir.clone()));

    let mut chain = Chain::new(mount);
    if opt.handle_assets {
        let assets = StaticAssets::new(downloaded_dir.clone(), "static".to_string());
        chain.link_after(assets);
    }
    chain.link_after(NotFoundResponsePath {
        path: downloaded_dir.join(Path::new("index.html")),
    });

    let listen = format!("0.0.0.0:{}", opt.port);
    println!("Start server {}", listen);
    Iron::new(chain).http(listen).unwrap();
}

fn downloaded_dir(
    vcs: String,
    user: String,
    project: String,
    build_num: u32,
    directory: Option<PathBuf>,
) -> PathBuf {
    let dest = format!(
        "{vcs}/{user}/{project}/{build_num}",
        vcs = vcs,
        user = user,
        project = project,
        build_num = build_num
    );
    match directory {
        Some(dir) => dir.join(dest),
        None => PathBuf::from(&dest),
    }
}

fn download_artifacts(
    vcs: String,
    user: String,
    project: String,
    build_num: u32,
    token: String,
    directory: Option<PathBuf>,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let dest_path = downloaded_dir(
        vcs.clone(),
        user.clone(),
        project.clone(),
        build_num,
        directory,
    );

    let circle_job = CircleCIBuild {
        vcs: vcs,
        user: user,
        project: project,
        build_num: build_num,
    };

    circle_job.download_artifacts(&token, dest_path.as_ref())?;

    Ok(PathBuf::from(dest_path))
}

struct CircleCIBuild {
    vcs: String,
    user: String,
    project: String,
    build_num: u32,
}

impl CircleCIBuild {
    fn artifacts_endpoint(&self) -> String {
        format!(
            "https://circleci.com/api/v1.1/project/{vcs}/{user}/{project}/{build_num}/artifacts?limit=20&offset=5&filter=completed",
            vcs=self.vcs, user=self.user, project=self.project, build_num=self.build_num
        )
    }

    fn artifact_urls(&self, token: &String) -> Result<Vec<String>, reqwest::Error> {
        let url =
            reqwest::Url::parse_with_params(&self.artifacts_endpoint(), &[("circle-token", token)])
                .unwrap();
        let artifacts: Vec<CircleCIArtifact> = reqwest::get(url)?.json()?;
        let urls = artifacts.iter().map(|x| x.url.clone()).collect();
        Ok(urls)
    }

    fn download_artifacts(
        &self,
        token: &String,
        destination: &Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let urls = self.artifact_urls(token)?;
        let common_len = urls.lcp().len();

        let client = reqwest::Client::new();
        for url in &urls {
            println!("Download {}", url);
            let url_with_token = reqwest::Url::parse_with_params(url, &[("circle-token", token)])?;
            let mut resp = client.get(url_with_token).send()?;

            let mut file = url.clone();
            file.replace_range(..common_len, "");

            let file_path = destination.join(Path::new(&file));
            let dir = file_path.parent().unwrap();
            fs::create_dir_all(dir).unwrap();
            let mut buf = BufWriter::new(File::create(file_path)?);

            resp.copy_to(&mut buf)?;
        }
        Ok(())
    }
}

fn lcp(str1: &String, str2: &String) -> String {
    let len1 = str1.len();
    let len2 = str2.len();
    let min_len = len1.min(len2);
    let mut i = 0;
    let mut utf8_i = 0;
    while i <= min_len && str1.get(0..i) == str2.get(0..i) {
        if str1.get(0..i).is_some() {
            utf8_i = i;
        }
        i += 1;
    }
    match str1.get(0..utf8_i) {
        Some(x) => String::from(x),
        None => String::new(),
    }
}

trait LCP {
    fn lcp(&self) -> String;
}

impl LCP for Vec<String> {
    fn lcp(&self) -> String {
        assert!(self.len() > 0);
        let mut prefix = self[0].clone();
        for s in &self[1..] {
            prefix = lcp(&prefix, s);
        }
        prefix
    }
}

#[derive(Deserialize, Debug)]
struct CircleCIArtifact {
    url: String,
}

struct NotFoundResponsePath {
    path: PathBuf,
}

impl AfterMiddleware for NotFoundResponsePath {
    fn catch(&self, _: &mut Request, err: IronError) -> IronResult<Response> {
        match err.response.status {
            Some(iron::status::NotFound) => {
                Ok(Response::with((iron::status::Ok, self.path.clone())))
            }
            _ => Err(err),
        }
    }
}

struct StaticAssets {
    dir_name: String,
    dir_path: PathBuf,
}

impl StaticAssets {
    fn new(server_root: PathBuf, dir_name: String) -> StaticAssets {
        let dir_path = server_root.join(&dir_name);
        StaticAssets { dir_name, dir_path }
    }
}

impl AfterMiddleware for StaticAssets {
    fn catch(&self, req: &mut Request, err: IronError) -> IronResult<Response> {
        match err.response.status {
            Some(iron::status::NotFound) if req.url.path().contains(&self.dir_name.as_str()) => {
                let url_path = req.url.path();
                let path_under_static = url_path
                    .split(|x| *x == self.dir_name)
                    .skip(1)
                    .next()
                    .unwrap();
                let path = self.dir_path.join(path_under_static.join("/"));
                Ok(Response::with((iron::status::Ok, path)))
            }
            _ => Err(err),
        }
    }
}
