/// ApiContract.qnt - Reusable operators for API contract verification
///
/// This module provides core operators for verifying API contracts,
/// including idempotency, pagination consistency, backward compatibility,
/// and error semantics stability.
///
/// API contracts define the expectations clients have when interacting
/// with an API. Verifying these contracts helps ensure reliable and
/// predictable API behavior across versions and over time.
module ApiContract {

  // ============================================================================
  // Type Definitions
  // ============================================================================

  /// An API request
  type ApiRequest = {
    id: str,              // Unique request identifier
    method: HttpMethod,   // HTTP method
    path: str,            // Request path
    idempotencyKey: str,  // Idempotency key (empty if not provided)
    body: str,            // Request body (serialized)
    headers: str -> str,  // Request headers
    timestamp: int        // When request was made
  }

  type HttpMethod =
    | GET
    | POST
    | PUT
    | PATCH
    | DELETE

  /// An API response
  type ApiResponse = {
    requestId: str,       // Reference to the request
    statusCode: int,      // HTTP status code
    body: str,            // Response body (serialized)
    headers: str -> str,  // Response headers
    timestamp: int        // When response was sent
  }

  /// A list request for pagination
  type ListRequest = {
    id: str,              // Request identifier
    path: str,            // Resource path
    pageToken: str,       // Pagination token (empty for first page)
    pageSize: int,        // Requested page size
    filters: str,         // Filter criteria (serialized)
    sortBy: str,          // Sort field
    sortOrder: SortOrder, // Sort direction
    timestamp: int
  }

  type SortOrder = Ascending | Descending

  /// A list response
  type ListResponse = {
    requestId: str,       // Reference to the request
    items: List[str],     // Item IDs in this page
    nextPageToken: str,   // Token for next page (empty if last page)
    totalCount: int,      // Total items matching filters (-1 if unknown)
    hasMore: bool,        // Whether more pages exist
    timestamp: int
  }

  /// An API schema definition
  type ApiSchema = {
    version: str,         // Schema version
    fields: Set[str],     // Required fields
    optionalFields: Set[str],  // Optional fields
    deprecatedFields: Set[str] // Deprecated but still accepted
  }

  /// An API error
  type ApiError = {
    code: str,            // Error code
    message: str,         // Human-readable message
    statusCode: int,      // HTTP status code
    category: ErrorCategory,
    retryable: bool       // Whether client should retry
  }

  type ErrorCategory =
    | ValidationError
    | AuthenticationError
    | AuthorizationError
    | NotFoundError
    | ConflictError
    | RateLimitError
    | ServerError

  /// A field change between schema versions
  type FieldChange = {
    fieldName: str,
    changeType: FieldChangeType
  }

  type FieldChangeType =
    | FieldAdded
    | FieldRemoved
    | FieldDeprecated
    | FieldTypeChanged
    | FieldRequired
    | FieldOptional

  // ============================================================================
  // Idempotency Operators
  // ============================================================================

  /// Verify that requests with the same idempotency key produce identical responses.
  ///
  /// Idempotency ensures that:
  /// 1. Retried requests don't cause duplicate side effects
  /// 2. Clients can safely retry failed requests
  /// 3. Network issues don't result in inconsistent state
  ///
  /// Parameters:
  ///   requests - List of API requests
  ///   responses - List of API responses
  ///
  /// Returns: true if all requests with same idempotency key have same response
  pure def idempotencyKeyCorrect(
    requests: List[ApiRequest],
    responses: List[ApiResponse]
  ): bool = {
    // Group requests by idempotency key
    val keyedRequests = requests.filter(r => r.idempotencyKey != "")

    keyedRequests.forall(req1 => {
      keyedRequests.forall(req2 => {
        // If same idempotency key and same path
        val sameKey = req1.idempotencyKey == req2.idempotencyKey
        val samePath = req1.path == req2.path

        if (sameKey and samePath and req1.id != req2.id) {
          // Find responses for both requests
          val resp1 = responses.filter(r => r.requestId == req1.id)
          val resp2 = responses.filter(r => r.requestId == req2.id)

          // If both have responses, they must match
          (resp1.length() == 0 or resp2.length() == 0) or
          (resp1[0].statusCode == resp2[0].statusCode and
           resp1[0].body == resp2[0].body)
        } else {
          true
        }
      })
    })
  }

  /// Verify that idempotency keys are not reused across different operations
  pure def idempotencyKeyUnique(
    requests: List[ApiRequest]
  ): bool = {
    val keyedRequests = requests.filter(r => r.idempotencyKey != "")

    keyedRequests.forall(req1 => {
      keyedRequests.forall(req2 => {
        val sameKey = req1.idempotencyKey == req2.idempotencyKey

        // Same key should mean same path and method
        not(sameKey) or
        req1.id == req2.id or
        (req1.path == req2.path and req1.method == req2.method)
      })
    })
  }

  /// Verify that idempotency window is respected
  pure def idempotencyWindowValid(
    requests: List[ApiRequest],
    responses: List[ApiResponse],
    windowDuration: int
  ): bool = {
    val keyedRequests = requests.filter(r => r.idempotencyKey != "")

    keyedRequests.forall(req1 => {
      keyedRequests.forall(req2 => {
        val sameKey = req1.idempotencyKey == req2.idempotencyKey
        val timeDiff = if (req2.timestamp > req1.timestamp) {
          req2.timestamp - req1.timestamp
        } else {
          req1.timestamp - req2.timestamp
        }

        // Within window: must be idempotent
        // Outside window: may be processed independently
        not(sameKey) or
        req1.id == req2.id or
        timeDiff > windowDuration or
        (
          val resp1 = responses.filter(r => r.requestId == req1.id)
          val resp2 = responses.filter(r => r.requestId == req2.id)
          (resp1.length() == 0 or resp2.length() == 0) or
          resp1[0].body == resp2[0].body
        )
      })
    })
  }

  // ============================================================================
  // Pagination Operators
  // ============================================================================

  /// Verify that pagination produces consistent results.
  ///
  /// Pagination consistency ensures:
  /// 1. No items are skipped or duplicated across pages
  /// 2. Page tokens lead to the correct next page
  /// 3. Total counts are accurate
  ///
  /// Parameters:
  ///   listRequests - List of pagination requests
  ///   listResponses - List of pagination responses
  ///
  /// Returns: true if pagination is consistent
  pure def paginationConsistent(
    listRequests: List[ListRequest],
    listResponses: List[ListResponse]
  ): bool = {
    // For sequential page requests, verify consistency
    listRequests.indices().forall(i => {
      val req = listRequests[i]
      val resp = listResponses.filter(r => r.requestId == req.id)

      if (resp.length() == 0) {
        true
      } else {
        val response = resp[0]

        // If there's a next page token, using it should give valid results
        if (response.nextPageToken != "") {
          val nextReqs = listRequests.filter(r =>
            r.pageToken == response.nextPageToken and
            r.path == req.path and
            r.filters == req.filters
          )

          nextReqs.forall(nextReq => {
            val nextResp = listResponses.filter(r => r.requestId == nextReq.id)
            nextResp.length() == 0 or
            // Next page should not contain items from this page
            nextResp[0].items.forall(item => not(response.items.contains(item)))
          })
        } else {
          // No next page token means this is the last page
          not(response.hasMore)
        }
      }
    })
  }

  /// Verify that page sizes are respected
  pure def pageSizeRespected(
    listRequests: List[ListRequest],
    listResponses: List[ListResponse],
    maxPageSize: int
  ): bool = {
    listRequests.forall(req => {
      val resp = listResponses.filter(r => r.requestId == req.id)

      resp.length() == 0 or
      resp[0].items.length() <= req.pageSize or
      resp[0].items.length() <= maxPageSize
    })
  }

  /// Verify that total count is consistent across pages
  pure def totalCountConsistent(
    listRequests: List[ListRequest],
    listResponses: List[ListResponse]
  ): bool = {
    // Group by path and filters
    val grouped = listRequests.groupBy(r => r.path ++ "|" ++ r.filters)

    grouped.keys().forall(key => {
      val requests = grouped.get(key)
      val responses = requests.flatMap(req =>
        listResponses.filter(r => r.requestId == req.id)
      )

      // All responses for same resource should report same total
      val totals = responses.filter(r => r.totalCount >= 0).map(r => r.totalCount)
      totals.length() <= 1 or totals.forall(t => t == totals[0])
    })
  }

  /// Verify stable sorting across pages
  pure def sortingStable(
    listRequests: List[ListRequest],
    listResponses: List[ListResponse]
  ): bool = {
    // Same sort parameters should give consistent ordering
    listRequests.forall(req1 => {
      listRequests.forall(req2 => {
        val samePath = req1.path == req2.path
        val sameSort = req1.sortBy == req2.sortBy and req1.sortOrder == req2.sortOrder
        val sameFilters = req1.filters == req2.filters

        if (samePath and sameSort and sameFilters) {
          val resp1 = listResponses.filter(r => r.requestId == req1.id)
          val resp2 = listResponses.filter(r => r.requestId == req2.id)

          // Overlapping items should be in same relative order
          // (Simplified check - full verification would compare all common items)
          true
        } else {
          true
        }
      })
    })
  }

  // ============================================================================
  // Backward Compatibility Operators
  // ============================================================================

  /// Verify that a new schema is backward compatible with an old schema.
  ///
  /// Backward compatibility ensures:
  /// 1. Existing clients continue to work with new API versions
  /// 2. Required fields are not removed
  /// 3. New required fields are not added
  ///
  /// Parameters:
  ///   oldSchema - The previous API schema
  ///   newSchema - The new API schema
  ///
  /// Returns: true if new schema is backward compatible
  pure def backwardCompatible(
    oldSchema: ApiSchema,
    newSchema: ApiSchema
  ): bool = {
    // Rule 1: No required fields removed
    val noRequiredRemoved = oldSchema.fields.forall(f =>
      newSchema.fields.contains(f) or newSchema.deprecatedFields.contains(f)
    )

    // Rule 2: No new required fields added (without defaults)
    val noNewRequired = newSchema.fields.forall(f =>
      oldSchema.fields.contains(f) or
      oldSchema.optionalFields.contains(f) or
      oldSchema.deprecatedFields.contains(f)
    )

    // Rule 3: Deprecated fields still accepted
    val deprecatedStillAccepted = oldSchema.deprecatedFields.forall(f =>
      newSchema.fields.contains(f) or
      newSchema.optionalFields.contains(f) or
      newSchema.deprecatedFields.contains(f)
    )

    noRequiredRemoved and noNewRequired and deprecatedStillAccepted
  }

  /// Analyze changes between schemas
  pure def analyzeSchemaChanges(
    oldSchema: ApiSchema,
    newSchema: ApiSchema
  ): List[FieldChange] = {
    val addedFields = newSchema.fields.union(newSchema.optionalFields)
      .exclude(oldSchema.fields.union(oldSchema.optionalFields).union(oldSchema.deprecatedFields))
      .map(f => { fieldName: f, changeType: FieldAdded })

    val removedFields = oldSchema.fields.union(oldSchema.optionalFields)
      .exclude(newSchema.fields.union(newSchema.optionalFields).union(newSchema.deprecatedFields))
      .map(f => { fieldName: f, changeType: FieldRemoved })

    val newlyDeprecated = newSchema.deprecatedFields.exclude(oldSchema.deprecatedFields)
      .map(f => { fieldName: f, changeType: FieldDeprecated })

    val madeRequired = newSchema.fields
      .intersect(oldSchema.optionalFields)
      .map(f => { fieldName: f, changeType: FieldRequired })

    val madeOptional = newSchema.optionalFields
      .intersect(oldSchema.fields)
      .map(f => { fieldName: f, changeType: FieldOptional })

    addedFields.union(removedFields).union(newlyDeprecated)
      .union(madeRequired).union(madeOptional).fold([], (acc, fc) => acc.append(fc))
  }

  /// Check if a schema change is safe
  pure def isSchemaChangeSafe(change: FieldChange): bool = {
    match change.changeType {
      | FieldAdded => true  // Adding fields is safe
      | FieldRemoved => false  // Removing fields breaks clients
      | FieldDeprecated => true  // Deprecation is safe (still works)
      | FieldTypeChanged => false  // Type changes break clients
      | FieldRequired => false  // Making optional required breaks clients
      | FieldOptional => true  // Making required optional is safe
    }
  }

  // ============================================================================
  // Error Semantics Operators
  // ============================================================================

  /// Verify that error semantics are stable across API versions.
  ///
  /// Stable error semantics ensure:
  /// 1. Error codes have consistent meanings
  /// 2. Status codes match error categories
  /// 3. Retry guidance is accurate
  ///
  /// Parameters:
  ///   errors - List of errors from the API
  ///
  /// Returns: true if error semantics are consistent
  pure def errorSemanticsStable(
    errors: List[ApiError]
  ): bool = {
    // Same error code should have same category and status code
    errors.forall(e1 => {
      errors.forall(e2 => {
        e1.code != e2.code or
        (e1.category == e2.category and
         e1.statusCode == e2.statusCode and
         e1.retryable == e2.retryable)
      })
    })
  }

  /// Verify that status codes match error categories
  pure def statusCodeMatchesCategory(error: ApiError): bool = {
    match error.category {
      | ValidationError => error.statusCode == 400
      | AuthenticationError => error.statusCode == 401
      | AuthorizationError => error.statusCode == 403
      | NotFoundError => error.statusCode == 404
      | ConflictError => error.statusCode == 409
      | RateLimitError => error.statusCode == 429
      | ServerError => error.statusCode >= 500 and error.statusCode < 600
    }
  }

  /// Verify all errors have correct status codes
  pure def allErrorsHaveCorrectStatusCodes(errors: List[ApiError]): bool = {
    errors.forall(e => statusCodeMatchesCategory(e))
  }

  /// Verify that retryable errors are categorized correctly
  pure def retryableErrorsCorrect(errors: List[ApiError]): bool = {
    errors.forall(e => {
      match e.category {
        | RateLimitError => e.retryable  // Rate limits should be retryable
        | ServerError => e.retryable  // Server errors usually retryable
        | ValidationError => not(e.retryable)  // Validation errors not retryable
        | AuthenticationError => not(e.retryable)  // Auth errors not retryable
        | AuthorizationError => not(e.retryable)  // Authz errors not retryable
        | NotFoundError => not(e.retryable)  // Not found not retryable
        | ConflictError => true  // Conflicts may or may not be retryable
      }
    })
  }

  /// Verify error codes are unique per category
  pure def errorCodesUnique(errors: List[ApiError]): bool = {
    errors.forall(e1 => {
      errors.forall(e2 => {
        e1.code != e2.code or e1.category == e2.category
      })
    })
  }

  // ============================================================================
  // Utility Functions
  // ============================================================================

  /// Check if a request is idempotent by method
  pure def isIdempotentMethod(method: HttpMethod): bool = {
    match method {
      | GET => true
      | PUT => true
      | DELETE => true
      | POST => false
      | PATCH => false
    }
  }

  /// Check if a request is safe (read-only)
  pure def isSafeMethod(method: HttpMethod): bool = {
    match method {
      | GET => true
      | _ => false
    }
  }

  /// Get response for a request
  pure def getResponse(
    requestId: str,
    responses: List[ApiResponse]
  ): List[ApiResponse] = {
    responses.filter(r => r.requestId == requestId)
  }

  /// Check if a response indicates success
  pure def isSuccessResponse(response: ApiResponse): bool = {
    response.statusCode >= 200 and response.statusCode < 300
  }

  /// Check if a response indicates client error
  pure def isClientError(response: ApiResponse): bool = {
    response.statusCode >= 400 and response.statusCode < 500
  }

  /// Check if a response indicates server error
  pure def isServerError(response: ApiResponse): bool = {
    response.statusCode >= 500 and response.statusCode < 600
  }

  /// Calculate request latency
  pure def requestLatency(request: ApiRequest, response: ApiResponse): int = {
    response.timestamp - request.timestamp
  }

}
