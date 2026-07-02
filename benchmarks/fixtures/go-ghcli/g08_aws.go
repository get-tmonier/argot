package cmd

import "github.com/aws/aws-sdk-go/service/s3"

func upload(svc *s3.S3, bucket, key string) error {
	_, err := svc.PutObject(&s3.PutObjectInput{Bucket: &bucket, Key: &key})
	return err
}
