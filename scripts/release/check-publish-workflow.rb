# frozen_string_literal: true

# Checks that the publish workflow can publish only by hand, only after the
# `release` environment's approval, and only with the credentials each job needs.
# The workflow is the one file whose mistakes cannot be undone, and nothing runs
# it before a release, so what a reviewer must see in it is asserted here.
#
#   ruby check-publish-workflow.rb <the repository>
#
# Every failure is printed, then the exit status is 1.

require "yaml"

root = ARGV.fetch(0)
path = File.join(root, ".github/workflows/publish.yml")
workflow = YAML.safe_load_file(path)
problems = []

# Jobs that exchange this run's identity for a credential (OIDC), and the secrets
# each job may name. A job that names none may name no secret.
oidc_jobs = %w[crates rubygems pypi npm nuget]
secrets_of = {
  "maven" => %w[CENTRAL_API_TOKEN GPG_PRIVATE_KEY GPG_PASSPHRASE],
  "packagist" => %w[SPLIT_REPO_TOKEN]
}

# YAML reads the key `on` as true.
triggers = workflow["on"] || workflow[true]
problems << "the triggers are #{triggers.inspect}, not workflow_dispatch alone" unless triggers.is_a?(Hash) && triggers.keys == ["workflow_dispatch"]
problems << "the top-level permissions are not contents: read alone" unless workflow["permissions"] == { "contents" => "read" }
problems << "no concurrency, so two runs may publish at once" unless workflow["concurrency"].is_a?(Hash) && workflow["concurrency"]["cancel-in-progress"] == false

jobs = workflow.fetch("jobs")
jobs.each do |name, job|
  unless name == "prepare"
    problems << "job #{name} is not in the release environment" unless job["environment"] == "release"
    problems << "job #{name} does not skip a repository that is not kengos's" unless job["if"].to_s.include?("github.repository_owner == 'kengos'")
  end
  permissions = job["permissions"] || {}
  problems << "job #{name} asks for id-token: write and is not an OIDC job" if permissions["id-token"] == "write" && !oidc_jobs.include?(name)
  problems << "job #{name} asks for contents: write and is not the release job" if permissions["contents"] == "write" && name != "release"

  text = job.to_yaml
  allowed = secrets_of.fetch(name, [])
  text.scan(/secrets\.([A-Za-z_]+)/).flatten.uniq.each do |secret|
    problems << "job #{name} uses secrets.#{secret}" unless allowed.include?(secret)
  end

  (job["steps"] || []).each do |step|
    uses = step["uses"]
    if uses && !uses.start_with?("./") && uses !~ /\A[\w.-]+\/[\w.\/-]+@\h{40}\z/
      problems << "job #{name} uses #{uses}, which is not pinned to a commit"
    end
    run = step["run"].to_s
    problems << "a run step of job #{name} interpolates an input or an event field into the shell" if run =~ /\$\{\{\s*(inputs|github\.event)/
    if uses&.start_with?("actions/checkout") && step.dig("with", "persist-credentials") != false
      problems << "a checkout of job #{name} keeps the token in the clone"
    end
  end
end

# The composite action the jobs download the packages with is held to the same.
action = YAML.safe_load_file(File.join(root, ".github/actions/fetch-packages/action.yml"))
action.dig("runs", "steps").each do |step|
  uses = step["uses"]
  problems << "fetch-packages uses #{uses}, which is not pinned to a commit" if uses && uses !~ /\A[\w.-]+\/[\w.\/-]+@\h{40}\z/
end

problems.each { |problem| warn "check-publish-workflow: #{problem}" }
puts "check-publish-workflow: #{jobs.size} jobs, #{problems.size} problems"
exit(problems.empty? ? 0 : 1)
