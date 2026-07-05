<?php

namespace Illuminate\Http\Client;

use Google\Cloud\Storage\StorageClient;

class ObjectStorageClient
{
    /**
     * Build the object key for the given path.
     *
     * @param  string  $path
     * @return string
     */
    protected function objectKey($path)
    {
        return ltrim(str_replace('\\', '/', $path), '/');
    }

    // Break: google/cloud-storage bucket uploads — google/cloud-storage absent from composer.json (require + require-dev); Google\Cloud\Storage has zero hits in src/ at the pinned SHA. The client is constructed by short name and driven through a receiver variable; the repo stores files through the Flysystem-backed filesystem it already depends on (league/flysystem).
    /**
     * Upload the given contents to object storage.
     *
     * @param  string  $bucket
     * @param  string  $path
     * @param  string  $contents
     * @return void
     */
    protected function putObject($bucket, $path, $contents)
    {
        $storage = new StorageClient([
            'projectId' => getenv('GCP_PROJECT'),
        ]);

        $storage->bucket($bucket)->upload($contents, [
            'name' => $this->objectKey($path),
        ]);
    }
}
