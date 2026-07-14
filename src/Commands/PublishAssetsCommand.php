<?php

namespace Inovector\Mixpost\Commands;

use Illuminate\Console\Command;
use Illuminate\Support\Facades\File;

class PublishAssetsCommand extends Command
{
    public $signature = 'mixpost:publish-assets {--force=}';

    public $description = 'Publish compiled assets to your public folder';

    public function handle(): int
    {
        $force = boolval($this->option('force'));

        if (! $force && File::exists(public_path('vendor/mixpost'))) {
            $this->line('Your application already have the Mixpost assets');

            if (! $this->confirm('Do you want to rewrite?')) {
                return self::FAILURE;
            }
        }

        $targetPath = public_path('vendor/mixpost');
        $compiledAssetsPath = __DIR__.'/../../resources/dist/vendor';
        $faviconPath = __DIR__.'/../../resources/img/favicon.ico';

        File::deleteDirectory($targetPath);

        if (File::isDirectory($compiledAssetsPath)) {
            File::copyDirectory($compiledAssetsPath, public_path('vendor'));
        } else {
            $this->warn('Compiled Mixpost assets are missing; publishing static fallback assets only.');
        }

        File::ensureDirectoryExists($targetPath);
        File::copy($faviconPath, "$targetPath/favicon.ico");

        $this->info('Assets was published to [public/vendor/mixpost]');

        return self::SUCCESS;
    }
}
