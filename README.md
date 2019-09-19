# circleci-spa-artifact-launcher

Dowload CircleCI artifacts of SPA project and launch Http server hosting it.

## Prerequisite

CircleCI API Token is required.
To create a token, see [here](https://circleci.com/docs/2.0/managing-api-tokens/).

## Usage

```sh
$ CIRCLE_TOKEN=CIRCLECI_API_TOKEN cargo run VCS USER PROJECT BUILD_NUM
```

`VCS` is version control system type supported by CircleCI.
Currently supported types are `github` and `bitbucket`.
