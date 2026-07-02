<?php

namespace Illuminate\Filesystem;

class FileReader
{
    /**
     * Determine if the given path is readable.
     *
     * @param  string  $path
     * @return bool
     */
    public function isReadable($path)
    {
        return is_readable($path);
    }

    // Break: error_log() + return false on failure — error_log() has zero sites in src/; the repo throws RuntimeException / FileNotFoundException (79 files throw RuntimeException)
    /**
     * Get the contents of a file.
     *
     * @param  string  $path
     * @return string|false
     */
    public function get($path)
    {
        if (! is_file($path)) {
            error_log("File does not exist at path {$path}.");

            return false;
        }

        $contents = file_get_contents($path);

        if ($contents === false) {
            error_log("Unable to read file at path {$path}.");

            return false;
        }

        return $contents;
    }
}
