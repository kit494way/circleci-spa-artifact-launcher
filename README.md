# circleci-spa-artifact-launcher

Dowload CircleCI artifacts of SPA project and launch Http server hosting it.

## Prerequisite

CircleCI API Token is required.
To create a token, see [here](https://circleci.com/docs/2.0/managing-api-tokens/).

## Build

```sh
$ cargo build
```

## Usage

```
$ csal --help
circleci-spa-artifact-launcher 0.1.0

USAGE:
    csal [FLAGS] [OPTIONS] <vcs> <user> <project> <build-num> --circle-token <circle-token>

FLAGS:
        --handle-assets    Return /static/path/to/assets file when /**/static/path/to/assets is requested. This may be
                           helpful when loading assets files from relative path.
    -h, --help             Prints help information
        --skip-download    Skip download artifacts, and launch server
    -V, --version          Prints version information

OPTIONS:
        --circle-token <circle-token>    CircleCI API Token [env: CIRCLE_TOKEN=]
        --directory <directory>          Default current directory
        --port <port>                     [default: 3000]

ARGS:
    <vcs>          VCS type supported by CircleCI. Currently 'github' or 'bitbucket'
    <user>         User or Organization
    <project>      CircleCI project name
    <build-num>    Build number of CircleCI job
```

### Run in docker

```sh
$ docker container run --rm -e CIRCLE_TOKEN=${CIRCLE_TOKEN} kit494way/circleci-spa-artifact-launcher <vcs> <user> <project> <build-num>
```
