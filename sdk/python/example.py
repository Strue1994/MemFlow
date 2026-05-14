from memflow import MemFlow, MemFlowConfig

mf = MemFlow(MemFlowConfig(api_key="sk-xxx"))
print(mf.workflows.list())
