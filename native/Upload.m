#import <Foundation/Foundation.h>
#include <stdatomic.h>
#include <time.h>
#include "Platform.h"

static uint64_t milliseconds(void) {
    struct timespec value;
    clock_gettime(CLOCK_MONOTONIC, &value);
    return (uint64_t)value.tv_sec * 1000 + (uint64_t)value.tv_nsec / 1000000;
}

// Error codes are private to github/native.rs: connection, timeout, source,
// cancellation, idle, oversized response, interrupted connection, setup.
@interface GattoUploadDelegate : NSObject <NSURLSessionDataDelegate> {
@public
    void *context;
    GattoProgress progress;
    uint64_t length;
    atomic_uint_fast64_t activity;
}
@property NSCondition *condition;
@property NSInputStream *input;
@property NSMutableData *body;
@property NSString *retryAfter;
@property NSInteger status;
@property NSInteger failure;
@property BOOL provided;
@property BOOL done;
@property BOOL invalidated;
@property uint64_t lastProgress;
@end

@implementation GattoUploadDelegate
- (void)URLSession:(NSURLSession *)session task:(NSURLSessionTask *)task
 needNewBodyStream:(void (^)(NSInputStream *))completion {
    [self.condition lock];
    NSInputStream *stream = self.provided ? nil : self.input;
    self.provided = YES;
    if (!stream && !self.failure) self.failure = 7;
    [self.condition unlock];
    completion(stream); // Supply the initial body once; never replay a POST.
}
- (void)URLSession:(NSURLSession *)session task:(NSURLSessionTask *)task
 needNewBodyStreamFromOffset:(int64_t)offset completionHandler:(void (^)(NSInputStream *))completion {
    [self.condition lock]; self.failure = 7; [self.condition unlock];
    completion(nil); // This endpoint has no resumable/replayable upload protocol.
}
- (void)URLSession:(NSURLSession *)session task:(NSURLSessionTask *)task
 willPerformHTTPRedirection:(NSHTTPURLResponse *)response newRequest:(NSURLRequest *)request
 completionHandler:(void (^)(NSURLRequest *))completion {
    completion(nil); // Never forward the bearer token to a redirected endpoint.
}
- (void)challenge:(NSURLAuthenticationChallenge *)challenge
 completion:(void (^)(NSURLSessionAuthChallengeDisposition, NSURLCredential *))completion {
    BOOL trust = [challenge.protectionSpace.authenticationMethod isEqualToString:NSURLAuthenticationMethodServerTrust];
    if (!trust) {
        [self.condition lock]; self.failure = 9; [self.condition unlock];
    }
    completion(trust ? NSURLSessionAuthChallengePerformDefaultHandling : NSURLSessionAuthChallengeCancelAuthenticationChallenge, nil);
}
- (void)URLSession:(NSURLSession *)session didReceiveChallenge:(NSURLAuthenticationChallenge *)challenge
 completionHandler:(void (^)(NSURLSessionAuthChallengeDisposition, NSURLCredential *))completion {
    [self challenge:challenge completion:completion];
}
- (void)URLSession:(NSURLSession *)session task:(NSURLSessionTask *)task
 didReceiveChallenge:(NSURLAuthenticationChallenge *)challenge
 completionHandler:(void (^)(NSURLSessionAuthChallengeDisposition, NSURLCredential *))completion {
    [self challenge:challenge completion:completion];
}
- (void)URLSession:(NSURLSession *)session dataTask:(NSURLSessionDataTask *)task
 didReceiveResponse:(NSURLResponse *)response
 completionHandler:(void (^)(NSURLSessionResponseDisposition))completion {
    if (![response isKindOfClass:NSHTTPURLResponse.class]) {
        [self.condition lock]; self.failure = 7; [self.condition unlock];
        completion(NSURLSessionResponseCancel);
        return;
    }
    NSHTTPURLResponse *http = (NSHTTPURLResponse *)response;
    self.status = http.statusCode;
    self.retryAfter = [http valueForHTTPHeaderField:@"Retry-After"];
    atomic_store(&activity, milliseconds());
    completion(NSURLSessionResponseAllow);
}
- (void)URLSession:(NSURLSession *)session dataTask:(NSURLSessionDataTask *)task
 didReceiveData:(NSData *)data {
    atomic_store(&activity, milliseconds());
    if (data.length > 65536 - self.body.length) {
        [self.condition lock]; self.failure = 6; [self.condition unlock];
        [task cancel];
    } else {
        [self.body appendData:data];
    }
}
- (void)URLSession:(NSURLSession *)session task:(NSURLSessionTask *)task
 didSendBodyData:(int64_t)bytes totalBytesSent:(int64_t)sent
 totalBytesExpectedToSend:(int64_t)expected {
    uint64_t now = milliseconds();
    atomic_store(&activity, now);
    if (sent >= 0 && (now - self.lastProgress >= 100 || (uint64_t)sent >= length)) {
        self.lastProgress = now;
        progress(context, MIN((uint64_t)sent, length), length);
    }
}
- (void)URLSession:(NSURLSession *)session task:(NSURLSessionTask *)task
 didCompleteWithError:(NSError *)error {
    [self.condition lock];
    if (!self.failure && error) {
        switch (error.code) {
            case NSURLErrorTimedOut: self.failure = 2; break;
            case NSURLErrorUserAuthenticationRequired: case NSURLErrorUserCancelledAuthentication:
                self.failure = 9; break;
            case NSURLErrorCannotFindHost: case NSURLErrorCannotConnectToHost:
            case NSURLErrorDNSLookupFailed: case NSURLErrorSecureConnectionFailed:
            case NSURLErrorServerCertificateUntrusted: case NSURLErrorServerCertificateHasBadDate:
            case NSURLErrorServerCertificateHasUnknownRoot: case NSURLErrorServerCertificateNotYetValid:
                self.failure = 1; break;
            default: self.failure = 7; break;
        }
    }
    self.done = YES;
    [self.condition broadcast];
    [self.condition unlock];
}
- (void)URLSession:(NSURLSession *)session didBecomeInvalidWithError:(NSError *)error {
    [self.condition lock];
    self.invalidated = YES;
    [self.condition broadcast];
    [self.condition unlock];
}
@end

@interface GattoUploadOutput : NSObject <NSStreamDelegate>
@property BOOL ready;
@property BOOL failed;
@end
@implementation GattoUploadOutput
- (void)stream:(NSStream *)stream handleEvent:(NSStreamEvent)event {
    if (event & NSStreamEventHasSpaceAvailable) self.ready = YES;
    if (event & NSStreamEventErrorOccurred) self.failed = YES;
}
@end

void GattoUpload(const char *url, const char *token, const char *mime,
                 const char *agent, uint64_t length, double total, double idle,
                 void *context, GattoRead read, GattoCanceled canceled,
                 GattoProgress progress, GattoUploadResult *result) {
    @autoreleasepool {
        memset(result, 0, sizeof(*result));
        NSURL *endpoint = [NSURL URLWithString:[NSString stringWithUTF8String:url]];
        if (!endpoint) { result->error = 8; return; }
        NSInputStream *input;
        NSOutputStream *output;
        [NSStream getBoundStreamsWithBufferSize:65536 inputStream:&input outputStream:&output];
        if (!input || !output) { result->error = 8; return; }
        GattoUploadDelegate *delegate = [GattoUploadDelegate new];
        delegate->context = context; delegate->progress = progress; delegate->length = length;
        atomic_init(&delegate->activity, milliseconds());
        delegate.condition = [NSCondition new];
        delegate.input = input;
        delegate.body = [NSMutableData data];
        NSURLSessionConfiguration *configuration = NSURLSessionConfiguration.ephemeralSessionConfiguration;
        configuration.URLCredentialStorage = nil;
        configuration.HTTPCookieStorage = nil;
        configuration.URLCache = nil;
        configuration.HTTPShouldSetCookies = NO;
        configuration.waitsForConnectivity = NO;
        configuration.timeoutIntervalForResource = total;
        // Our monotonic watchdog owns idle classification; avoid racing it
        // against Foundation's independently reset request-idle timer.
        configuration.timeoutIntervalForRequest = MAX(idle,total);
        NSOperationQueue *queue = [NSOperationQueue new];
        queue.maxConcurrentOperationCount = 1;
        NSURLSession *session = [NSURLSession sessionWithConfiguration:configuration delegate:delegate delegateQueue:queue];
        NSMutableURLRequest *request = [NSMutableURLRequest requestWithURL:endpoint];
        request.HTTPMethod = @"POST";
        request.HTTPShouldHandleCookies = NO;
        request.cachePolicy = NSURLRequestReloadIgnoringLocalCacheData;
        [request setValue:[@"Bearer " stringByAppendingString:[NSString stringWithUTF8String:token]] forHTTPHeaderField:@"Authorization"];
        [request setValue:@"application/json" forHTTPHeaderField:@"Accept"];
        [request setValue:[NSString stringWithUTF8String:mime] forHTTPHeaderField:@"Content-Type"];
        [request setValue:[NSString stringWithFormat:@"%llu", (unsigned long long)length] forHTTPHeaderField:@"Content-Length"];
        [request setValue:[NSString stringWithUTF8String:agent] forHTTPHeaderField:@"User-Agent"];
        NSURLSessionUploadTask *task = [session uploadTaskWithStreamedRequest:request];
        GattoUploadOutput *writer = [GattoUploadOutput new];
        output.delegate = writer;
        NSRunLoop *runLoop = NSRunLoop.currentRunLoop;
        [output scheduleInRunLoop:runLoop forMode:NSDefaultRunLoopMode];
        [output open];
        [task resume];
        uint8_t buffer[65536];
        size_t pending = 0, offset = 0;
        uint64_t produced = 0, start = milliseconds();
        BOOL closed = NO;
        for (;;) {
            [delegate.condition lock];
            BOOL done = delegate.done;
            NSInteger failure = delegate.failure;
            [delegate.condition unlock];
            if (done) break; // A received success wins a simultaneous cancel.
            uint64_t now = milliseconds();
            if (!failure && canceled(context)) failure = 4;
            if (!failure && now - start >= total * 1000) failure = 2;
            uint64_t lastActivity = atomic_load(&delegate->activity);
            if (!failure && now >= lastActivity && now - lastActivity >= idle * 1000) failure = 5;
            if (!failure && writer.failed) failure = 3;
            if (failure) {
                [delegate.condition lock];
                if (!delegate.done && !delegate.failure) delegate.failure = failure;
                [delegate.condition unlock];
                [task cancel];
            } else if (!closed && writer.ready) {
                if (offset == pending) {
                    intptr_t count = read(context, buffer, sizeof(buffer));
                    if (count < 0 || (count == 0 && produced != length) ||
                        (uint64_t)count > length - produced) {
                        [delegate.condition lock]; delegate.failure = 3; [delegate.condition unlock];
                        [task cancel];
                    } else if (count == 0) {
                        [output close]; closed = YES;
                    } else {
                        pending = (size_t)count; offset = 0; produced += (uint64_t)count;
                    }
                }
                if (!closed && offset < pending) {
                    writer.ready = NO; // One write per stream-readiness event.
                    NSInteger count = [output write:buffer + offset maxLength:pending - offset];
                    if (count < 0) writer.failed = YES;
                    else offset += (size_t)count;
                    if (offset == pending && produced == length) {
                        uint8_t extra;
                        if (read(context, &extra, 1) != 0) {
                            [delegate.condition lock]; delegate.failure = 3; [delegate.condition unlock];
                            [task cancel];
                        }
                        [output close]; closed = YES;
                    }
                }
            }
            if (![runLoop runMode:NSDefaultRunLoopMode beforeDate:[NSDate dateWithTimeIntervalSinceNow:0.01]]) {
                [NSThread sleepForTimeInterval:0.005]; // No stream source after EOF; don't busy-spin awaiting the response.
            }
        }
        [output close];
        [output removeFromRunLoop:runLoop forMode:NSDefaultRunLoopMode];
        output.delegate = nil;
        [session invalidateAndCancel];
        // cancel is asynchronous. Wait for invalidation before freeing Rust's
        // callback context; the serial delegate queue cannot call it afterwards.
        [delegate.condition lock];
        while (!delegate.invalidated) [delegate.condition wait];
        [delegate.condition unlock];
        result->status = (uint16_t)delegate.status;
        result->error = (int)delegate.failure;
        result->body_length = delegate.body.length;
        if (result->body_length) {
            result->body = malloc(result->body_length);
            if (result->body) memcpy(result->body, delegate.body.bytes, result->body_length);
            else { result->body_length = 0; result->error = 8; }
        }
        if (delegate.retryAfter) result->retry_after = strdup(delegate.retryAfter.UTF8String);
    }
}

void GattoUploadResultFree(GattoUploadResult *result) {
    free(result->body);
    free(result->retry_after);
    memset(result, 0, sizeof(*result));
}
