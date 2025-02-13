#!/usr/bin/env node
import { Command } from 'commander';
import fs from 'fs-extra';
import path from 'path';
import { fileURLToPath } from 'url';

// Get the package root directory
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const PACKAGE_ROOT = path.join(__dirname, '..');

const program = new Command();

program
  .name('create-sovereign-agent')
  .description('CLI to create or modify agent repos with Sovereign Enclave scaffold');

// create-sovereign-agent new <projectName>
program
  .command('new <projectName>')
  .description('Create a new directory with Sovereign Enclave scaffold')
  .action(async (projectName) => {
    try {
      await createNewProject(projectName);
    } catch (err) {
      console.error('Error:', err.message || err);
      process.exit(1);
    }
  });

// create-sovereign-agent add <sourceAgentDir>
program
  .command('add <sourceAgentDir>')
  .description('Add Sovereign Enclave scaffold to an existing agent folder')
  .option('--docker-url <string>', 'Full Docker image URL to inject into Dockerfiles')
  .action(async (sourceAgentDir, options) => {
    try {
      await wrapAgentFolder(sourceAgentDir, options);
    } catch (err) {
      console.error('Error:', err.message || err);
      process.exit(1);
    }
  });

program.parse(process.argv);

/**
 * Wrap the user's existing agent code into a new folder containing:
 * - agent/ (copied from sourceAgentDir)
 * - Dockerfile, Dockerfile.agent, config.json, start.sh, run_enclave.sh, Makefile, etc.
 */
async function wrapAgentFolder(sourceAgentDir, options) {
  const absoluteTarget = path.resolve(process.cwd(), sourceAgentDir);

  // Ensure source exists
  if (!(await fs.pathExists(absoluteTarget))) {
    throw new Error(`Source folder "${sourceAgentDir}" does not exist.`);
  }

  console.log(`\nAdding Sovereign Enclave scaffold to "${sourceAgentDir}"...\n`);

  // Copy template files from templates directory
  const enclaveTemplateDir = path.join(__dirname, 'templates', 'enclave');
  const dockerfileAgent = path.join(__dirname, 'templates', 'Dockerfile.agent');

  if (!(await fs.pathExists(enclaveTemplateDir))) {
    throw new Error(`Missing enclave template directory: ${enclaveTemplateDir}`);
  }
  if (!(await fs.pathExists(dockerfileAgent))) {
    throw new Error(`Missing Dockerfile.agent template: ${dockerfileAgent}`);
  }

  // Copy enclave folder
  await fs.copy(enclaveTemplateDir, path.join(absoluteTarget, 'enclave'), { overwrite: true });
  
  // Copy Dockerfile.agent to root
  await fs.copy(dockerfileAgent, path.join(absoluteTarget, 'Dockerfile.agent'), { overwrite: true });

  // If user provided --docker-url, do string replacement on placeholders
  if (options.dockerUrl) {
    const templateFiles = await fs.readdir(enclaveTemplateDir);
    for (const file of templateFiles) {
      const destFile = path.join(absoluteTarget, file);

      if ((await fs.stat(destFile)).isFile()) {
        let content = await fs.readFile(destFile, 'utf-8');
        if (content.includes('{{DOCKER_IMAGE_URL}}')) {
          content = content.replace(/{{DOCKER_IMAGE_URL}}/g, options.dockerUrl);
          await fs.writeFile(destFile, content, 'utf-8');
        }
      }
    }

    console.log(`Injected --docker-url (${options.dockerUrl}) into Dockerfiles.`);
  } else {
    console.log(`No --docker-url provided; left {{DOCKER_IMAGE_URL}} placeholders as-is.`);
  }

  // Print final instructions
  console.log(`\nSuccess! Your enclave wrapper has been added to "${sourceAgentDir}".\n`);
  console.log(`Template files added:\n`);
  console.log(`  Dockerfile.agent (in root)`);
  console.log(`  enclave/ folder containing:`);
  console.log(`    Dockerfile, config.json, start.sh, run_enclave.sh, Makefile, etc.\n`);

  console.log(`You can now build images, reference your Docker URL, etc.\n`);
  console.log(`Example steps:\n`);
  console.log(`  cd ${sourceAgentDir}`);
  console.log(`  # If you want to build the agent image (Dockerfile.agent) and push`);
  console.log(`  docker build -t <your-repo/agent:tag> -f Dockerfile.agent .`);
  console.log(`  docker push <your-repo/agent:tag>\n`);
  console.log(`  # Then build the runner/enclave image (Dockerfile)`);
  console.log(`  docker build -t <your-repo/runner:tag> -f Dockerfile .`);
  console.log(`  docker push <your-repo/runner:tag>\n`);
  console.log(`  # Then run it!`);
  console.log(`  docker run --rm <your-repo/runner:tag>\n`);
}

async function createNewProject(projectName) {
  const targetDir = path.resolve(process.cwd(), projectName);

  // Ensure target doesn't exist
  if (await fs.pathExists(targetDir)) {
    throw new Error(`Directory "${projectName}" already exists.`);
  }

  console.log(`\nCreating new Sovereign Enclave project in "${projectName}"...\n`);

  // Create the directory
  await fs.mkdirp(targetDir);

  // Copy template files from templates directory
  const enclaveDir = path.join(__dirname, 'templates', 'enclave');
  const dockerfileAgent = path.join(__dirname, 'templates', 'Dockerfile.agent');

  if (!(await fs.pathExists(enclaveDir))) {
    throw new Error(`Missing enclave directory: ${enclaveDir}`);
  }
  if (!(await fs.pathExists(dockerfileAgent))) {
    throw new Error(`Missing Dockerfile.agent: ${dockerfileAgent}`);
  }

  // Copy enclave folder and Dockerfile.agent
  await fs.copy(enclaveDir, path.join(targetDir, 'enclave'));
  await fs.copy(dockerfileAgent, path.join(targetDir, 'Dockerfile.agent'));

  console.log(`\nSuccess! Created new project in "${projectName}".\n`);
  console.log(`Template files added:\n`);
  console.log(`  Dockerfile.agent (in root)`);
  console.log(`  enclave/ folder containing:`);
  console.log(`    Dockerfile, config.json, start.sh, run_enclave.sh, Makefile, etc.\n`);
  console.log(`Next steps:\n`);
  console.log(`  cd ${projectName}`);
  console.log(`  git init`);
  console.log(`  # Start adding your agent code!\n`);
}