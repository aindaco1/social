#!/usr/bin/env node

import { createHash } from 'node:crypto';
import { existsSync, readFileSync, statSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const projectRoot = path.resolve(scriptDirectory, '..');
const optional = process.argv.includes('--optional');
const sourceRoot = path.join(projectRoot, 'resources', 'desktop', 'public', 'litert', 'models');
const distRoot = path.join(projectRoot, 'resources', 'desktop', 'dist', 'litert', 'models');
const roots = [sourceRoot];

if (existsSync(path.join(projectRoot, 'resources', 'desktop', 'dist'))) {
    roots.push(distRoot);
}

let failed = false;
let validatedModels = 0;

function fail(message) {
    console.error(message);
    failed = true;
}

function relative(filePath) {
    return path.relative(projectRoot, filePath);
}

function sha256(filePath) {
    return createHash('sha256').update(readFileSync(filePath)).digest('hex');
}

function requireString(value, label) {
    if (typeof value !== 'string' || !value.trim()) {
        fail(`[invalid] ${label} must be a non-empty string`);
        return '';
    }

    return value.trim();
}

function safeModelPath(root, filePath, label) {
    const value = requireString(filePath, label);

    if (!value) {
        return '';
    }

    if (path.isAbsolute(value) || value.split(/[\\/]/).includes('..')) {
        fail(`[invalid] ${label} must be a relative path inside the models directory`);
        return '';
    }

    const absolutePath = path.resolve(root, value);

    if (!absolutePath.startsWith(`${path.resolve(root)}${path.sep}`)) {
        fail(`[invalid] ${label} escapes the models directory`);
        return '';
    }

    return absolutePath;
}

function validateFile(root, file, modelId, index) {
    const label = `${modelId}.files[${index}].path`;
    const filePath = safeModelPath(root, file?.path, label);

    if (!filePath) {
        return;
    }

    if (!existsSync(filePath)) {
        fail(`[missing] ${relative(filePath)}`);
        return;
    }

    const expectedSize = Number(file?.size);
    const expectedSha = requireString(file?.sha256, `${modelId}.files[${index}].sha256`);

    if (!Number.isSafeInteger(expectedSize) || expectedSize <= 0) {
        fail(`[invalid] ${modelId}.files[${index}].size must be a positive integer`);
    } else {
        const actualSize = statSync(filePath).size;

        if (actualSize !== expectedSize) {
            fail(`[invalid] ${relative(filePath)} size ${actualSize} does not match ${expectedSize}`);
        }
    }

    if (expectedSha && !/^[a-f0-9]{64}$/.test(expectedSha)) {
        fail(`[invalid] ${modelId}.files[${index}].sha256 must be a lowercase SHA-256 hex digest`);
    } else if (expectedSha) {
        const actualSha = sha256(filePath);

        if (actualSha !== expectedSha) {
            fail(`[invalid] ${relative(filePath)} SHA-256 ${actualSha} does not match ${expectedSha}`);
        }
    }
}

function validateModel(root, model, index) {
    const id = requireString(model?.id, `models[${index}].id`);

    if (!id) {
        return;
    }

    for (const field of [
        'name',
        'feature',
        'runtime',
        'format',
        'version',
        'source_url',
        'license',
        'license_url',
        'license_file',
    ]) {
        requireString(model?.[field], `${id}.${field}`);
    }

    if (model.runtime !== 'litert') {
        fail(`[invalid] ${id}.runtime must be "litert" for the MVP LiteRT model bundle`);
    }

    if (model.format !== 'tflite') {
        fail(`[invalid] ${id}.format must be "tflite" for the MVP LiteRT model bundle`);
    }

    if (!String(model.source_url || '').startsWith('https://')) {
        fail(`[invalid] ${id}.source_url must be HTTPS`);
    }

    if (!String(model.license_url || '').startsWith('https://')) {
        fail(`[invalid] ${id}.license_url must be HTTPS`);
    }

    const licensePath = safeModelPath(root, model.license_file, `${id}.license_file`);

    if (licensePath && !existsSync(licensePath)) {
        fail(`[missing] ${relative(licensePath)}`);
    }

    if (!Array.isArray(model.files) || model.files.length === 0) {
        fail(`[invalid] ${id}.files must include the model, metadata, and license files`);
    } else {
        const hasModel = model.files.some((file) => file?.kind === 'model');
        const hasLicense = model.files.some((file) => file?.kind === 'license');

        if (!hasModel) {
            fail(`[invalid] ${id}.files must include one file with kind "model"`);
        }

        if (!hasLicense) {
            fail(`[invalid] ${id}.files must include one file with kind "license"`);
        }

        model.files.forEach((file, fileIndex) => validateFile(root, file, id, fileIndex));
    }

    if (model.feature === 'image_upscaling') {
        const inputShape = model.inputs?.image?.shape;
        const outputShape = model.outputs?.upscaled_image?.shape;

        if (!Array.isArray(inputShape) || inputShape.join('x') !== '1x128x128x3') {
            fail(`[invalid] ${id}.inputs.image.shape must be 1x128x128x3`);
        }

        if (!Array.isArray(outputShape) || outputShape.join('x') !== '1x512x512x3') {
            fail(`[invalid] ${id}.outputs.upscaled_image.shape must be 1x512x512x3`);
        }
    }

    validatedModels += 1;
}

function validateRoot(root) {
    const manifestPath = path.join(root, 'manifest.json');

    if (!existsSync(manifestPath)) {
        if (!optional) {
            fail(`[missing] ${relative(manifestPath)}`);
        }

        return;
    }

    let manifest;

    try {
        manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
    } catch (error) {
        fail(`[invalid] ${relative(manifestPath)} could not be parsed: ${error.message}`);
        return;
    }

    if (manifest.schema_version !== 1) {
        fail(`[invalid] ${relative(manifestPath)} schema_version must be 1`);
    }

    if (!Array.isArray(manifest.models) || manifest.models.length === 0) {
        fail(`[invalid] ${relative(manifestPath)} must include at least one model`);
        return;
    }

    manifest.models.forEach((model, index) => validateModel(root, model, index));
}

for (const root of roots) {
    validateRoot(root);
}

if (failed) {
    process.exit(1);
}

if (validatedModels === 0) {
    console.log('No local AI model manifest found.');
} else {
    console.log(`Local AI model manifest validated for ${validatedModels} model bundle(s).`);
}
