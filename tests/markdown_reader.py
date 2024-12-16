from utils import cargo_run, goto_root, mk_and_cd_tmp_dir, write_string

# fixed by 8e06403f
sample1 = '''
this is a sentence. `this is an unterminated
code span`.
'''

# fixed by bf1906cb
sample2 = "\n".join([
    "a" * 3500,
    *(["aaaa"] * 200),
])

# fixed by bf1906cb
sample3 = "\n".join([
    "a" * 6000,
    "aa",
    "aa",
])

# fixed by 05714cd0
sample4 = """

<!--
The file is auto-generated from the Go source code of the component using a generic
[generator](https://github.com/kubernetes-sigs/reference-docs/). To learn how
to generate the reference documentation, please read
[Contributing to the reference documentation](/docs/contribute/generate-ref-docs/).
To update the reference content, please follow the
[Contributing upstream](/docs/contribute/generate-ref-docs/contribute-upstream/)
guide. You can file document formatting bugs against the
[reference-docs](https://github.com/kubernetes-sigs/reference-docs/) project.
-->


## {{% heading "synopsis" %}}


The Kubernetes scheduler is a control plane process which assigns
Pods to Nodes. The scheduler determines which Nodes are valid placements for
each Pod in the scheduling queue according to constraints and available
resources. The scheduler then ranks each valid Node and binds the Pod to a
suitable Node. Multiple different schedulers may be used within a cluster;
kube-scheduler is the reference implementation.
See [scheduling](https://kubernetes.io/docs/concepts/scheduling-eviction/)
for more information about scheduling and the kube-scheduler component.

```
kube-scheduler [flags]
```

<tr>
<td colspan="2">--feature-gates colonSeparatedMultimapStringString</td>
</tr>
<tr>
<td></td><td style="line-height: 130%; word-wrap: break-word;"><p>Comma-separated list of component:key=value pairs that describe feature gates for alpha/experimental features of different components.<br/>If the component is not specified, defaults to &quot;kube&quot;. This flag can be repeatedly invoked. For example: --feature-gates 'wardle:featureA=true,wardle:featureB=false' --feature-gates 'kube:featureC=true'Options are:<br/>kube:APIResponseCompression=true|false (BETA - default=true)<br/>kube:APIServerIdentity=true|false (BETA - default=true)<br/>kube:APIServerTracing=true|false (BETA - default=true)<br/>kube:APIServingWithRoutine=true|false (ALPHA - default=false)<br/>kube:AllAlpha=true|false (ALPHA - default=false)<br/>kube:AllBeta=true|false (BETA - default=false)<br/>kube:AnonymousAuthConfigurableEndpoints=true|false (ALPHA - default=false)<br/>kube:AnyVolumeDataSource=true|false (BETA - default=true)<br/>kube:AuthorizeNodeWithSelectors=true|false (ALPHA - default=false)<br/>kube:AuthorizeWithSelectors=true|false (ALPHA - default=false)<br/>kube:CPUManagerPolicyAlphaOptions=true|false (ALPHA - default=false)<br/>kube:CPUManagerPolicyBetaOptions=true|false (BETA - default=true)<br/>kube:CPUManagerPolicyOptions=true|false (BETA - default=true)<br/>kube:CRDValidationRatcheting=true|false (BETA - default=true)<br/>kube:CSIMigrationPortworx=true|false (BETA - default=true)<br/>kube:CSIVolumeHealth=true|false (ALPHA - default=false)<br/>kube:CloudControllerManagerWebhook=true|false (ALPHA - default=false)<br/>kube:ClusterTrustBundle=true|false (ALPHA - default=false)<br/>kube:ClusterTrustBundleProjection=true|false (ALPHA - default=false)<br/>kube:ComponentSLIs=true|false (BETA - default=true)<br/>kube:ConcurrentWatchObjectDecode=true|false (BETA - default=false)<br/>kube:ConsistentListFromCache=true|false (BETA - default=true)<br/>kube:ContainerCheckpoint=true|false (BETA - default=true)<br/>kube:ContextualLogging=true|false (BETA - default=true)<br/>kube:CoordinatedLeaderElection=true|false (ALPHA - default=false)<br/>kube:CronJobsScheduledAnnotation=true|false (BETA - default=true)<br/>kube:CrossNamespaceVolumeDataSource=true|false (ALPHA - default=false)<br/>kube:CustomCPUCFSQuotaPeriod=true|false (ALPHA - default=false)<br/>kube:CustomResourceFieldSelectors=true|false (BETA - default=true)<br/>kube:DRAControlPlaneController=true|false (ALPHA - default=false)<br/>kube:DisableAllocatorDualWrite=true|false (ALPHA - default=false)<br/>kube:DisableNodeKubeProxyVersion=true|false (BETA - default=true)<br/>kube:DynamicResourceAllocation=true|false (ALPHA - default=false)<br/>kube:EventedPLEG=true|false (ALPHA - default=false)<br/>kube:GracefulNodeShutdown=true|false (BETA - default=true)<br/>kube:GracefulNodeShutdownBasedOnPodPriority=true|false (BETA - default=true)<br/>kube:HPAScaleToZero=true|false (ALPHA - default=false)<br/>kube:HonorPVReclaimPolicy=true|false (BETA - default=true)<br/>kube:ImageMaximumGCAge=true|false (BETA - default=true)<br/>kube:ImageVolume=true|false (ALPHA - default=false)<br/>kube:InPlacePodVerticalScaling=true|false (ALPHA - default=false)<br/>kube:InTreePluginPortworxUnregister=true|false (ALPHA - default=false)<br/>kube:InformerResourceVersion=true|false (ALPHA - default=false)<br/>kube:JobBackoffLimitPerIndex=true|false (BETA - default=true)<br/>kube:JobManagedBy=true|false (ALPHA - default=false)<br/>kube:JobPodReplacementPolicy=true|false (BETA - default=true)<br/>kube:JobSuccessPolicy=true|false (BETA - default=true)<br/>kube:KubeletCgroupDriverFromCRI=true|false (BETA - default=true)<br/>kube:KubeletInUserNamespace=true|false (ALPHA - default=false)<br/>kube:KubeletPodResourcesDynamicResources=true|false (ALPHA - default=false)<br/>kube:KubeletPodResourcesGet=true|false (ALPHA - default=false)<br/>kube:KubeletSeparateDiskGC=true|false (BETA - default=true)<br/>kube:KubeletTracing=true|false (BETA - default=true)<br/>kube:LoadBalancerIPMode=true|false (BETA - default=true)<br/>kube:LocalStorageCapacityIsolationFSQuotaMonitoring=true|false (BETA - default=false)<br/>kube:LoggingAlphaOptions=true|false (ALPHA - default=false)<br/>kube:LoggingBetaOptions=true|false (BETA - default=true)<br/>kube:MatchLabelKeysInPodAffinity=true|false (BETA - default=true)<br/>kube:MatchLabelKeysInPodTopologySpread=true|false (BETA - default=true)<br/>kube:MaxUnavailableStatefulSet=true|false (ALPHA - default=false)<br/>kube:MemoryManager=true|false (BETA - default=true)<br/>kube:MemoryQoS=true|false (ALPHA - default=false)<br/>kube:MultiCIDRServiceAllocator=true|false (BETA - default=false)<br/>kube:MutatingAdmissionPolicy=true|false (ALPHA - default=false)<br/>kube:NFTablesProxyMode=true|false (BETA - default=true)<br/>kube:NodeInclusionPolicyInPodTopologySpread=true|false (BETA - default=true)<br/>kube:NodeLogQuery=true|false (BETA - default=false)<br/>kube:NodeSwap=true|false (BETA - default=true)<br/>kube:OpenAPIEnums=true|false (BETA - default=true)<br/>kube:PodAndContainerStatsFromCRI=true|false (ALPHA - default=false)<br/>kube:PodDeletionCost=true|false (BETA - default=true)<br/>kube:PodIndexLabel=true|false (BETA - default=true)<br/>kube:PodLifecycleSleepAction=true|false (BETA - default=true)<br/>kube:PodReadyToStartContainersCondition=true|false (BETA - default=true)<br/>kube:PortForwardWebsockets=true|false (BETA - default=true)<br/>kube:ProcMountType=true|false (BETA - default=false)<br/>kube:QOSReserved=true|false (ALPHA - default=false)<br/>kube:RecoverVolumeExpansionFailure=true|false (ALPHA - default=false)<br/>kube:RecursiveReadOnlyMounts=true|false (BETA - default=true)<br/>kube:RelaxedEnvironmentVariableValidation=true|false (ALPHA - default=false)<br/>kube:ReloadKubeletServerCertificateFile=true|false (BETA - default=true)<br/>kube:ResilientWatchCacheInitialization=true|false (BETA - default=true)<br/>kube:ResourceHealthStatus=true|false (ALPHA - default=false)<br/>kube:RetryGenerateName=true|false (BETA - default=true)<br/>kube:RotateKubeletServerCertificate=true|false (BETA - default=true)<br/>kube:RuntimeClassInImageCriApi=true|false (ALPHA - default=false)<br/>kube:SELinuxMount=true|false (ALPHA - default=false)<br/>kube:SELinuxMountReadWriteOncePod=true|false (BETA - default=true)<br/>kube:SchedulerQueueingHints=true|false (BETA - default=false)<br/>kube:SeparateCacheWatchRPC=true|false (BETA - default=true)<br/>kube:SeparateTaintEvictionController=true|false (BETA - default=true)<br/>kube:ServiceAccountTokenJTI=true|false (BETA - default=true)<br/>kube:ServiceAccountTokenNodeBinding=true|false (BETA - default=true)<br/>kube:ServiceAccountTokenNodeBindingValidation=true|false (BETA - default=true)<br/>kube:ServiceAccountTokenPodNodeInfo=true|false (BETA - default=true)<br/>kube:ServiceTrafficDistribution=true|false (BETA - default=true)<br/>kube:SidecarContainers=true|false (BETA - default=true)<br/>kube:SizeMemoryBackedVolumes=true|false (BETA - default=true)<br/>kube:StatefulSetAutoDeletePVC=true|false (BETA - default=true)<br/>kube:StorageNamespaceIndex=true|false (BETA - default=true)<br/>kube:StorageVersionAPI=true|false (ALPHA - default=false)<br/>kube:StorageVersionHash=true|false (BETA - default=true)<br/>kube:StorageVersionMigrator=true|false (ALPHA - default=false)<br/>kube:StrictCostEnforcementForVAP=true|false (BETA - default=false)<br/>kube:StrictCostEnforcementForWebhooks=true|false (BETA - default=false)<br/>kube:StructuredAuthenticationConfiguration=true|false (BETA - default=true)<br/>kube:StructuredAuthorizationConfiguration=true|false (BETA - default=true)<br/>kube:SupplementalGroupsPolicy=true|false (ALPHA - default=false)<br/>kube:TopologyAwareHints=true|false (BETA - default=true)<br/>kube:TopologyManagerPolicyAlphaOptions=true|false (ALPHA - default=false)<br/>kube:TopologyManagerPolicyBetaOptions=true|false (BETA - default=true)<br/>kube:TopologyManagerPolicyOptions=true|false (BETA - default=true)<br/>kube:TranslateStreamCloseWebsocketRequests=true|false (BETA - default=true)<br/>kube:UnauthenticatedHTTP2DOSMitigation=true|false (BETA - default=true)<br/>kube:UnknownVersionInteroperabilityProxy=true|false (ALPHA - default=false)<br/>kube:UserNamespacesPodSecurityStandards=true|false (ALPHA - default=false)<br/>kube:UserNamespacesSupport=true|false (BETA - default=false)<br/>kube:VolumeAttributesClass=true|false (BETA - default=false)<br/>kube:VolumeCapacityPriority=true|false (ALPHA - default=false)<br/>kube:WatchCacheInitializationPostStartHook=true|false (BETA - default=false)<br/>kube:WatchFromStorageWithoutResourceVersion=true|false (BETA - default=false)<br/>kube:WatchList=true|false (ALPHA - default=false)<br/>kube:WatchListClient=true|false (BETA - default=false)<br/>kube:WinDSR=true|false (ALPHA - default=false)<br/>kube:WinOverlay=true|false (BETA - default=true)<br/>kube:WindowsHostNetwork=true|false (ALPHA - default=true)</p></td>
</tr>

<tr>
<td colspan="2">-h, --help</td>
</tr>
<tr>
<td></td><td style="line-height: 130%; word-wrap: break-word;"><p>help for kube-scheduler</p></td>
</tr>






"""

# fixed by fe26f156
sample5 = """
2. some title
  - some sentence
  - another sentence
    - `<|media(PATH/TO/YOUR/MEDIA/FILE)|>`
    - `<|raw_media(png:BASE64_VALUE_OF_YOUR_MEDIA_FILE)|>`. For now, it supports `png`, `jpeg`, `gif` and `webp`.

You'll find pdl files in 2 places: your local ragit repo and ragit's git repo.

1. If you have initialized a ragit repo, you'll find pdl files in `./.ragit/prompts`. Modify the files and run `rag build` or `rag query` to see how LLM behaves differently. Make sure to `rag config --set dump_log true` so that you can see the conversations.
2. You can also find `prompts/` in ragit's git repo. This is the default value for prompts. If your local tests on your new prompts are satisfiable, please commit the new prompts.
"""

def markdown_reader():
    goto_root()
    mk_and_cd_tmp_dir()

    cargo_run(["init"])
    cargo_run(["config", "--set", "model", "dummy"])

    write_string("sample1.md", sample1)
    cargo_run(["config", "--set", "chunk_size", "4000"])
    cargo_run(["config", "--set", "slide_len", "1000"])
    cargo_run(["add", "sample1.md"])
    cargo_run(["build"], timeout=20.0)
    cargo_run(["check"])

    write_string("sample2.md", sample2)
    cargo_run(["config", "--set", "chunk_size", "4000"])
    cargo_run(["config", "--set", "slide_len", "1000"])
    cargo_run(["add", "sample2.md"])
    cargo_run(["build"], timeout=20.0)
    cargo_run(["check"])

    write_string("sample3.md", sample3)
    cargo_run(["config", "--set", "chunk_size", "4000"])
    cargo_run(["config", "--set", "slide_len", "1000"])
    cargo_run(["add", "sample3.md"])
    cargo_run(["build"], timeout=20.0)
    cargo_run(["check"])

    write_string("sample4.md", sample4)
    cargo_run(["config", "--set", "chunk_size", "4000"])
    cargo_run(["config", "--set", "slide_len", "1000"])
    cargo_run(["add", "sample4.md"])
    cargo_run(["build"], timeout=20.0)
    cargo_run(["check"])

    write_string("sample5.md", sample5)
    cargo_run(["config", "--set", "chunk_size", "4000"])
    cargo_run(["config", "--set", "slide_len", "1000"])
    cargo_run(["add", "sample5.md"])
    cargo_run(["build"], timeout=20.0)
    cargo_run(["check"])

    # If pdl-escaping was successful, summay of its chunk must have this substring
    assert "<|media(PATH/TO/YOUR/MEDIA/FILE)|>" in cargo_run(["ls-chunks", "sample5.md"], stdout=True)
