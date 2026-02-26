/// idempotency.qnt - Full specification for API idempotency key handling
///
/// This specification models an API system that uses idempotency keys to ensure
/// that duplicate or retried requests produce the same result as the original.
/// This is critical for payment processing, order creation, and other operations
/// where duplicate execution would cause problems.
///
/// Key properties verified:
/// - SameKeyMeansIdenticalResponse: Same idempotency key always returns same response
/// - NoSideEffectsOnRetry: Retried requests don't cause additional state changes
/// - IdempotencyKeyExpiration: Keys expire after a configured window
/// - ConcurrentRequestHandling: Concurrent requests with same key are handled safely
module idempotency {

  import ApiContract.*

  // ============================================================================
  // Constants
  // ============================================================================

  /// Idempotency key expiration window (in time units)
  const IDEMPOTENCY_WINDOW: int = 86400  // 24 hours

  /// Maximum concurrent requests per key
  const MAX_CONCURRENT_PER_KEY: int = 1

  /// Set of client identifiers
  const CLIENTS: Set[str] = Set("client1", "client2", "client3")

  /// Set of API paths
  const API_PATHS: Set[str] = Set("/orders", "/payments", "/transfers")

  /// Set of idempotency keys for testing
  const IDEMPOTENCY_KEYS: Set[str] = Set("key1", "key2", "key3", "key4", "key5")

  // ============================================================================
  // State Variables
  // ============================================================================

  /// All requests received by the system
  var requests: List[ApiRequest]

  /// All responses sent by the system
  var responses: List[ApiResponse]

  /// Idempotency key store: maps key to stored response
  var idempotencyStore: str -> StoredIdempotency

  /// In-flight requests: keys currently being processed
  var inFlightKeys: Set[str]

  /// System clock
  var clock: int

  /// Request counter for generating unique IDs
  var requestCounter: int

  /// Resource state (simplified: maps path to version)
  var resourceState: str -> int

  /// Side effect log (to verify no duplicate side effects)
  var sideEffectLog: List[SideEffect]

  // ============================================================================
  // Additional Type Definitions
  // ============================================================================

  /// Stored idempotency information
  type StoredIdempotency = {
    key: str,             // The idempotency key
    requestHash: str,     // Hash of the original request body
    response: ApiResponse,// The stored response
    createdAt: int,       // When the key was first used
    expiresAt: int        // When the key expires
  }

  /// A side effect caused by a request
  type SideEffect = {
    requestId: str,       // Request that caused the side effect
    idempotencyKey: str,  // Idempotency key (if any)
    effectType: str,      // Type of side effect
    resourcePath: str,    // Affected resource
    timestamp: int
  }

  /// Processing state for a request
  type ProcessingState =
    | NewRequest
    | DuplicateRequest
    | KeyInFlight
    | KeyExpired

  // ============================================================================
  // Helper Functions
  // ============================================================================

  /// Generate a unique request ID
  pure def generateRequestId(counter: int): str = {
    "req_" ++ counter.toString()
  }

  /// Hash a request for comparison (simplified: use body)
  pure def hashRequest(request: ApiRequest): str = {
    request.method.toString() ++ "|" ++ request.path ++ "|" ++ request.body
  }

  /// Check if an idempotency key is expired
  pure def isKeyExpired(stored: StoredIdempotency, currentTime: int): bool = {
    currentTime > stored.expiresAt
  }

  /// Get the processing state for a request
  pure def getProcessingState(
    key: str,
    requestHash: str,
    store: str -> StoredIdempotency,
    inFlight: Set[str],
    currentTime: int
  ): ProcessingState = {
    if (inFlight.contains(key)) {
      KeyInFlight
    } else if (store.keys().contains(key)) {
      val stored = store.get(key)
      if (isKeyExpired(stored, currentTime)) {
        KeyExpired
      } else if (stored.requestHash == requestHash) {
        DuplicateRequest
      } else {
        // Same key, different request body - conflict
        DuplicateRequest  // Return stored response
      }
    } else {
      NewRequest
    }
  }

  // ============================================================================
  // State Predicates
  // ============================================================================

  /// Check if a key exists and is valid
  def keyExistsAndValid(key: str): bool = {
    idempotencyStore.keys().contains(key) and
    not(isKeyExpired(idempotencyStore.get(key), clock))
  }

  /// Check if a key is currently in-flight
  def keyInFlight(key: str): bool = {
    inFlightKeys.contains(key)
  }

  /// Get stored response for a key
  def getStoredResponse(key: str): List[ApiResponse] = {
    if (idempotencyStore.keys().contains(key)) {
      [idempotencyStore.get(key).response]
    } else {
      []
    }
  }

  // ============================================================================
  // Actions
  // ============================================================================

  /// Initialize the system
  action Init: bool = all {
    requests' = [],
    responses' = [],
    idempotencyStore' = Map(),
    inFlightKeys' = Set(),
    clock' = 0,
    requestCounter' = 0,
    resourceState' = Map(),
    sideEffectLog' = []
  }

  /// Send a new request (client action)
  action SendRequest(
    clientId: str,
    method: HttpMethod,
    path: str,
    idempotencyKey: str,
    body: str
  ): bool = {
    val requestId = generateRequestId(requestCounter)
    val newRequest: ApiRequest = {
      id: requestId,
      method: method,
      path: path,
      idempotencyKey: idempotencyKey,
      body: body,
      headers: Map().set("X-Client-ID", clientId),
      timestamp: clock
    }

    all {
      requests' = requests.append(newRequest),
      responses' = responses,
      idempotencyStore' = idempotencyStore,
      inFlightKeys' = if (idempotencyKey != "") {
        inFlightKeys.union(Set(idempotencyKey))
      } else {
        inFlightKeys
      },
      clock' = clock,
      requestCounter' = requestCounter + 1,
      resourceState' = resourceState,
      sideEffectLog' = sideEffectLog
    }
  }

  /// Process a request and generate response (server action)
  action ReceiveResponse(requestId: str): bool = {
    val matchingRequests = requests.filter(r => r.id == requestId)

    if (matchingRequests.length() == 0) {
      all {
        requests' = requests,
        responses' = responses,
        idempotencyStore' = idempotencyStore,
        inFlightKeys' = inFlightKeys,
        clock' = clock,
        requestCounter' = requestCounter,
        resourceState' = resourceState,
        sideEffectLog' = sideEffectLog
      }
    } else {
      val request = matchingRequests[0]
      val key = request.idempotencyKey
      val requestHash = hashRequest(request)

      // Determine processing state
      val state = getProcessingState(
        key,
        requestHash,
        idempotencyStore,
        inFlightKeys,
        clock
      )

      match state {
        | DuplicateRequest => {
          // Return stored response
          val storedResponse = idempotencyStore.get(key).response
          val responseWithNewId: ApiResponse = {
            ...storedResponse,
            requestId: requestId,
            timestamp: clock
          }

          all {
            requests' = requests,
            responses' = responses.append(responseWithNewId),
            idempotencyStore' = idempotencyStore,
            inFlightKeys' = inFlightKeys.exclude(Set(key)),
            clock' = clock + 1,
            requestCounter' = requestCounter,
            resourceState' = resourceState,
            sideEffectLog' = sideEffectLog  // No new side effects!
          }
        }

        | KeyInFlight => {
          // Return 409 Conflict - request already in progress
          val conflictResponse: ApiResponse = {
            requestId: requestId,
            statusCode: 409,
            body: "{\"error\": \"Request with this idempotency key is already in progress\"}",
            headers: Map(),
            timestamp: clock
          }

          all {
            requests' = requests,
            responses' = responses.append(conflictResponse),
            idempotencyStore' = idempotencyStore,
            inFlightKeys' = inFlightKeys,
            clock' = clock + 1,
            requestCounter' = requestCounter,
            resourceState' = resourceState,
            sideEffectLog' = sideEffectLog
          }
        }

        | KeyExpired => {
          // Process as new request (key expired)
          val currentVersion = resourceState.getOrElse(request.path, 0)
          val newVersion = currentVersion + 1

          val successResponse: ApiResponse = {
            requestId: requestId,
            statusCode: 201,
            body: "{\"id\": \"" ++ requestId ++ "\", \"version\": " ++ newVersion.toString() ++ "}",
            headers: Map(),
            timestamp: clock
          }

          val newSideEffect: SideEffect = {
            requestId: requestId,
            idempotencyKey: key,
            effectType: "resource_created",
            resourcePath: request.path,
            timestamp: clock
          }

          val newStoredIdempotency: StoredIdempotency = {
            key: key,
            requestHash: requestHash,
            response: successResponse,
            createdAt: clock,
            expiresAt: clock + IDEMPOTENCY_WINDOW
          }

          all {
            requests' = requests,
            responses' = responses.append(successResponse),
            idempotencyStore' = if (key != "") {
              idempotencyStore.set(key, newStoredIdempotency)
            } else {
              idempotencyStore
            },
            inFlightKeys' = inFlightKeys.exclude(Set(key)),
            clock' = clock + 1,
            requestCounter' = requestCounter,
            resourceState' = resourceState.set(request.path, newVersion),
            sideEffectLog' = sideEffectLog.append(newSideEffect)
          }
        }

        | NewRequest => {
          // Process new request
          val currentVersion = resourceState.getOrElse(request.path, 0)
          val newVersion = currentVersion + 1

          val successResponse: ApiResponse = {
            requestId: requestId,
            statusCode: 201,
            body: "{\"id\": \"" ++ requestId ++ "\", \"version\": " ++ newVersion.toString() ++ "}",
            headers: Map(),
            timestamp: clock
          }

          val newSideEffect: SideEffect = {
            requestId: requestId,
            idempotencyKey: key,
            effectType: "resource_created",
            resourcePath: request.path,
            timestamp: clock
          }

          val newStoredIdempotency: StoredIdempotency = {
            key: key,
            requestHash: requestHash,
            response: successResponse,
            createdAt: clock,
            expiresAt: clock + IDEMPOTENCY_WINDOW
          }

          all {
            requests' = requests,
            responses' = responses.append(successResponse),
            idempotencyStore' = if (key != "") {
              idempotencyStore.set(key, newStoredIdempotency)
            } else {
              idempotencyStore
            },
            inFlightKeys' = inFlightKeys.exclude(Set(key)),
            clock' = clock + 1,
            requestCounter' = requestCounter,
            resourceState' = resourceState.set(request.path, newVersion),
            sideEffectLog' = sideEffectLog.append(newSideEffect)
          }
        }
      }
    }
  }

  /// Retry a previous request (client action)
  action RetryRequest(originalRequestId: str): bool = {
    val originalRequests = requests.filter(r => r.id == originalRequestId)

    if (originalRequests.length() == 0) {
      all {
        requests' = requests,
        responses' = responses,
        idempotencyStore' = idempotencyStore,
        inFlightKeys' = inFlightKeys,
        clock' = clock,
        requestCounter' = requestCounter,
        resourceState' = resourceState,
        sideEffectLog' = sideEffectLog
      }
    } else {
      val original = originalRequests[0]
      val newRequestId = generateRequestId(requestCounter)

      // Create retry request with same idempotency key
      val retryRequest: ApiRequest = {
        id: newRequestId,
        method: original.method,
        path: original.path,
        idempotencyKey: original.idempotencyKey,
        body: original.body,
        headers: original.headers,
        timestamp: clock
      }

      all {
        requests' = requests.append(retryRequest),
        responses' = responses,
        idempotencyStore' = idempotencyStore,
        inFlightKeys' = if (original.idempotencyKey != "" and not(inFlightKeys.contains(original.idempotencyKey))) {
          inFlightKeys.union(Set(original.idempotencyKey))
        } else {
          inFlightKeys
        },
        clock' = clock,
        requestCounter' = requestCounter + 1,
        resourceState' = resourceState,
        sideEffectLog' = sideEffectLog
      }
    }
  }

  /// Advance time (system action)
  action AdvanceTime(delta: int): bool = all {
    requests' = requests,
    responses' = responses,
    idempotencyStore' = idempotencyStore,
    inFlightKeys' = inFlightKeys,
    clock' = clock + delta,
    requestCounter' = requestCounter,
    resourceState' = resourceState,
    sideEffectLog' = sideEffectLog
  }

  /// Clean up expired idempotency keys (system action)
  action CleanupExpiredKeys: bool = {
    val validKeys = idempotencyStore.keys().filter(k =>
      not(isKeyExpired(idempotencyStore.get(k), clock))
    )

    val newStore = validKeys.fold(Map(), (acc, k) =>
      acc.set(k, idempotencyStore.get(k))
    )

    all {
      requests' = requests,
      responses' = responses,
      idempotencyStore' = newStore,
      inFlightKeys' = inFlightKeys,
      clock' = clock,
      requestCounter' = requestCounter,
      resourceState' = resourceState,
      sideEffectLog' = sideEffectLog
    }
  }

  /// Non-deterministic step
  action step: bool = {
    nondet clientId = oneOf(CLIENTS)
    nondet method = oneOf(Set(POST, PUT))
    nondet path = oneOf(API_PATHS)
    nondet key = oneOf(IDEMPOTENCY_KEYS.union(Set("")))
    nondet body = oneOf(Set("body1", "body2", "body3"))
    nondet requestId = oneOf(Set("req_0", "req_1", "req_2", "req_3", "req_4"))
    nondet timeDelta = oneOf(Set(1, 100, 1000, 100000))

    any {
      SendRequest(clientId, method, path, key, body),
      ReceiveResponse(requestId),
      RetryRequest(requestId),
      AdvanceTime(timeDelta),
      CleanupExpiredKeys
    }
  }

  // ============================================================================
  // Invariants
  // ============================================================================

  /// INV1: SameKeyMeansIdenticalResponse
  /// All requests with the same idempotency key must receive identical responses
  /// (except for request ID and timestamp)
  val SameKeyMeansIdenticalResponse: bool = {
    requests.forall(req1 => {
      requests.forall(req2 => {
        val sameKey = req1.idempotencyKey == req2.idempotencyKey
        val keyNotEmpty = req1.idempotencyKey != ""

        if (sameKey and keyNotEmpty and req1.id != req2.id) {
          val resp1 = responses.filter(r => r.requestId == req1.id)
          val resp2 = responses.filter(r => r.requestId == req2.id)

          // If both have responses (not 409 conflict), bodies must match
          (resp1.length() == 0 or resp2.length() == 0) or
          resp1[0].statusCode == 409 or
          resp2[0].statusCode == 409 or
          resp1[0].body == resp2[0].body
        } else {
          true
        }
      })
    })
  }

  /// INV2: NoSideEffectsOnRetry
  /// Duplicate requests with the same idempotency key must not create additional side effects
  val NoSideEffectsOnRetry: bool = {
    // For each idempotency key, there should be at most one side effect
    val keyedRequests = requests.filter(r => r.idempotencyKey != "")
    val keys = keyedRequests.map(r => r.idempotencyKey).toSet()

    keys.forall(key => {
      val keyEffects = sideEffectLog.filter(e => e.idempotencyKey == key)
      keyEffects.length() <= 1
    })
  }

  /// INV3: StoredResponseMatchesOriginal
  /// The stored response must match what was originally returned
  val StoredResponseMatchesOriginal: bool = {
    idempotencyStore.keys().forall(key => {
      val stored = idempotencyStore.get(key)
      val originalRequest = requests.filter(r =>
        r.idempotencyKey == key and
        hashRequest(r) == stored.requestHash
      )

      originalRequest.length() == 0 or
      responses.exists(r =>
        r.body == stored.response.body and
        r.statusCode == stored.response.statusCode
      )
    })
  }

  /// INV4: InFlightKeysConsistent
  /// A key is in-flight only if there's an unprocessed request with that key
  val InFlightKeysConsistent: bool = {
    inFlightKeys.forall(key => {
      val keyRequests = requests.filter(r => r.idempotencyKey == key)
      val keyResponses = responses.filter(r =>
        keyRequests.exists(req => req.id == r.requestId)
      )
      keyRequests.length() > keyResponses.length()
    })
  }

  /// INV5: ExpirationWindowRespected
  /// Stored idempotency entries have valid expiration times
  val ExpirationWindowRespected: bool = {
    idempotencyStore.keys().forall(key => {
      val stored = idempotencyStore.get(key)
      stored.expiresAt == stored.createdAt + IDEMPOTENCY_WINDOW
    })
  }

  /// INV6: RequestBodyMismatchHandled
  /// Requests with same key but different body should return the stored response
  /// (not create new resources)
  val RequestBodyMismatchHandled: bool = {
    requests.forall(req1 => {
      requests.forall(req2 => {
        val sameKey = req1.idempotencyKey == req2.idempotencyKey
        val differentBody = req1.body != req2.body
        val keyNotEmpty = req1.idempotencyKey != ""

        if (sameKey and differentBody and keyNotEmpty) {
          val effects1 = sideEffectLog.filter(e => e.requestId == req1.id)
          val effects2 = sideEffectLog.filter(e => e.requestId == req2.id)

          // At most one should have caused a side effect
          effects1.length() == 0 or effects2.length() == 0
        } else {
          true
        }
      })
    })
  }

  /// Combined safety invariant
  val Safety: bool = {
    SameKeyMeansIdenticalResponse and
    NoSideEffectsOnRetry and
    ExpirationWindowRespected
  }

  // ============================================================================
  // Test Scenarios
  // ============================================================================

  /// Test: First request with idempotency key succeeds
  run testFirstRequestSucceeds = {
    Init
    .then(SendRequest("client1", POST, "/orders", "key1", "order_data"))
    .then(ReceiveResponse("req_0"))
    .then(all {
      assert(responses.length() == 1),
      assert(responses[0].statusCode == 201),
      assert(sideEffectLog.length() == 1)
    })
  }

  /// Test: Retry with same key returns same response
  run testRetryReturnsSameResponse = {
    Init
    .then(SendRequest("client1", POST, "/orders", "key1", "order_data"))
    .then(ReceiveResponse("req_0"))
    .then(RetryRequest("req_0"))
    .then(ReceiveResponse("req_1"))
    .then(all {
      assert(SameKeyMeansIdenticalResponse),
      assert(NoSideEffectsOnRetry),
      assert(responses[0].body == responses[1].body),
      assert(sideEffectLog.length() == 1)  // Only one side effect
    })
  }

  /// Test: Different keys create separate resources
  run testDifferentKeysCreateSeparate = {
    Init
    .then(SendRequest("client1", POST, "/orders", "key1", "order1"))
    .then(ReceiveResponse("req_0"))
    .then(SendRequest("client1", POST, "/orders", "key2", "order2"))
    .then(ReceiveResponse("req_1"))
    .then(all {
      assert(responses.length() == 2),
      assert(sideEffectLog.length() == 2)  // Two separate side effects
    })
  }

  /// Test: Concurrent requests with same key
  run testConcurrentRequests = {
    Init
    .then(SendRequest("client1", POST, "/orders", "key1", "order1"))
    .then(SendRequest("client2", POST, "/orders", "key1", "order1"))  // Same key
    .then(ReceiveResponse("req_0"))
    .then(ReceiveResponse("req_1"))  // Should get conflict or same response
    .then(all {
      assert(SameKeyMeansIdenticalResponse),
      assert(NoSideEffectsOnRetry)
    })
  }

  /// Test: Expired key allows new request
  run testExpiredKeyAllowsNewRequest = {
    Init
    .then(SendRequest("client1", POST, "/orders", "key1", "order1"))
    .then(ReceiveResponse("req_0"))
    .then(AdvanceTime(IDEMPOTENCY_WINDOW + 1))  // Expire the key
    .then(CleanupExpiredKeys)
    .then(SendRequest("client1", POST, "/orders", "key1", "order2"))  // Same key, after expiry
    .then(ReceiveResponse("req_1"))
    .then(all {
      // After expiration, a new request with same key should be processed
      assert(sideEffectLog.length() == 2)  // Two side effects (key expired)
    })
  }

}
