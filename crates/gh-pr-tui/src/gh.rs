use anyhow::{Result, bail};
use octocrab::{Octocrab, params};

use crate::{Repo, pr::Pr};

pub async fn comment(octocrab: &Octocrab, repo: &Repo, pr: &Pr, body: &str) -> Result<()> {
    let issue = octocrab.issues(&repo.org, &repo.repo);
    issue.create_comment(pr.number as _, body).await?;

    Ok(())
}

/// Merges a pull request.
pub async fn merge(octocrab: &Octocrab, repo: &Repo, pr: &Pr) -> Result<()> {
    let page = octocrab
        .pulls(&repo.org, &repo.repo)
        .merge(pr.number as _)
        .method(params::pulls::MergeMethod::Squash)
        .send()
        .await?;

    if !page.merged {
        bail!(
            "Failed to merge PR #{} in {}/{}",
            pr.number,
            repo.org,
            repo.repo
        );
    }

    Ok(())
}
