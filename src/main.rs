extern crate reqwest;
extern crate serde;
extern crate serde_json;

use std::env;
use std::fs;
use std::fs::File;
use std::io::BufWriter;
use std::io::Write;
use std::path::Path;
use std::str::FromStr;

use serde::Deserialize;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let circle_token = env::var("CIRCLE_TOKEN").expect("missing CIRCLE_TOKEN environemt variable");
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 5 {
        writeln!(
            std::io::stderr(),
            "Usage: {} VCS USER PROJECT BUILD_NUM",
            Path::new(&args[0]).file_name().unwrap().to_str().unwrap()
        )
        .unwrap();
        std::process::exit(1);
    }

    let vcs = &args[1];
    let user = &args[2];
    let project = &args[3];
    let build_num = u32::from_str(&args[4]).unwrap();

    let circle_job = CircleCIBuild {
        vcs: String::from(vcs),
        user: String::from(user),
        project: String::from(project),
        build_num: build_num,
    };

    let dest = format!(
        "{vcs}/{user}/{project}/{build_num}",
        vcs = circle_job.vcs,
        user = circle_job.user,
        project = circle_job.project,
        build_num = circle_job.build_num
    );

    circle_job.download_artifacts(&circle_token, Path::new(&dest)).unwrap();

    Ok(())
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
