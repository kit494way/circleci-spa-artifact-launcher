extern crate iron;
extern crate mount;
extern crate reqwest;
extern crate serde;
extern crate serde_json;
extern crate staticfile;

use std::env;
use std::fs;
use std::fs::File;
use std::io::BufWriter;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;

use iron::prelude::*;
use iron::AfterMiddleware;
use mount::Mount;
use serde::Deserialize;
use staticfile::Static;

fn main() {
    let circle_token = env::var("CIRCLE_TOKEN").unwrap_or_else(|_| {
        writeln!(std::io::stderr(), "missing CIRCLE_TOKEN environemt variable.").unwrap();
        std::process::exit(1);
    });

    let port = env::var("PORT").unwrap_or("3000".to_string());

    let vcs = nth_arg_or_exit(1);
    let user = nth_arg_or_exit(2);
    let project = nth_arg_or_exit(3);
    let build_num = u32::from_str(nth_arg_or_exit(4).as_str()).unwrap();

    let downloaded_dir = download_artifacts(vcs, user, project, build_num, circle_token).unwrap();

    let mut mount = Mount::new();
    mount.mount("/", Static::new(downloaded_dir.clone()));

    let mut chain = Chain::new(mount);
    chain.link_after(NotFoundResponsePath {
        path: downloaded_dir.join(Path::new("index.html")),
    });

    println!("Start server localhost:{}", port);
    Iron::new(chain).http(format!("localhost:{}", port)).unwrap();
}

fn nth_arg_or_exit(nth: usize) -> String {
    let arg = std::env::args().nth(nth);
    if arg.is_none() {
        print_usage_and_exit();
    }
    arg.unwrap()
}

fn print_usage_and_exit() {
    let program = std::env::args().nth(0).unwrap();
    let program_name = Path::new(&program).file_name().and_then(|f| f.to_str()).unwrap();
    writeln!(
        std::io::stderr(),
        "Usage: {} VCS USER PROJECT BUILD_NUM",
        program_name
    )
    .unwrap();
    std::process::exit(1);
}

fn download_artifacts(
    vcs: String,
    user: String,
    project: String,
    build_num: u32,
    token: String,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let circle_job = CircleCIBuild {
        vcs: vcs,
        user: user,
        project: project,
        build_num: build_num,
    };

    let dest = format!(
        "{vcs}/{user}/{project}/{build_num}",
        vcs = circle_job.vcs,
        user = circle_job.user,
        project = circle_job.project,
        build_num = circle_job.build_num
    );
    let dest_path = Path::new(&dest);

    circle_job.download_artifacts(&token, dest_path)?;

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
