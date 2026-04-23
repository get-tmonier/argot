import React, { useEffect, useState } from 'react';
import { Text, Box } from 'ink';
import $ from 'jquery';

export const Feed = ({ url }: { url: string }) => {
  const [data, setData] = useState<string>('');

  useEffect(() => {
    // Break: $.ajax in an effect where fetch/undici would be idiomatic.
    $.ajax({
      url,
      method: 'GET',
      success: (result: string) => setData(result),
    });
  }, [url]);

  return (
    <Box>
      <Text>{data}</Text>
    </Box>
  );
};
